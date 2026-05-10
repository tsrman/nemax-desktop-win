fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();

        // ── Иконка и версия через windres напрямую ──
        let icon_path = std::path::Path::new(&manifest_dir)
            .join("assets")
            .join("icon.ico");

        let rc_path = std::path::Path::new(&out_dir).join("resource.rc");
        let res_path = std::path::Path::new(&out_dir).join("resource.o");

        let version = env!("CARGO_PKG_VERSION");
        let rc_content = format!(
            "#pragma code_page(65001)\n\
             1 VERSIONINFO\n\
             FILEOS 0x40004\n\
             FILESUBTYPE 0x0\n\
             FILEFLAGSMASK 0x3f\n\
             FILEFLAGS 0x0\n\
             PRODUCTVERSION {v_comma}\n\
             FILETYPE 0x1\n\
             FILEVERSION {v_comma}\n\
             {{\n\
             BLOCK \"StringFileInfo\"\n\
             {{\n\
             BLOCK \"000004b0\"\n\
             {{\n\
             VALUE \"ProductName\", \"neMAX\"\n\
             VALUE \"ProductVersion\", \"{v}\"\n\
             VALUE \"FileDescription\", \"neMAX\"\n\
             VALUE \"FileVersion\", \"{v}\"\n\
             }}\n\
             }}\n\
             BLOCK \"VarFileInfo\" {{\n\
             VALUE \"Translation\", 0x0, 0x04b0\n\
             }}\n\
             }}\n\
             1 ICON \"{icon}\"",
            v = version,
            v_comma = version.replace('.', ","),
            icon = icon_path.display(),
        );

        std::fs::write(&rc_path, &rc_content).unwrap();

        // Компилируем .rc → .o через windres
        let windres = std::env::var("WINDRES").unwrap_or_else(|_| "windres".into());
        let status = std::process::Command::new(&windres)
            .arg(rc_path)
            .arg(&res_path)
            .status()
            .expect("Failed to run windres");

        if !status.success() {
            panic!("windres failed");
        }

        // Линкуем .o напрямую (для GNU тулчейна)
        // В отличие от winres, передаём object file, а не static library
        println!("cargo:rustc-link-arg={}", res_path.display());

        // ── WebView2Loader.dll ──
        let target_dir = std::path::Path::new(&out_dir)
            .parent().unwrap()
            .parent().unwrap()
            .parent().unwrap();

        let dll_src = target_dir
            .join("build")
            .read_dir()
            .ok()
            .and_then(|mut dir| {
                dir.find_map(|e| {
                    let p = e.ok()?.path();
                    if p.file_name()?.to_str()?.starts_with("webview2-com-sys-") {
                        let dll = p.join("out").join("x64").join("WebView2Loader.dll");
                        if dll.exists() { Some(dll) } else { None }
                    } else {
                        None
                    }
                })
            });

        if let Some(ref src) = dll_src {
            let dll_dest = target_dir.join("release").join("WebView2Loader.dll");
            let _ = std::fs::copy(src, &dll_dest);
        }
    }
}
