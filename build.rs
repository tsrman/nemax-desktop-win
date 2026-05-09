fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();

        // Копируем WebView2Loader.dll из артефактов webview2-com-sys
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_dir = std::path::Path::new(&out_dir)
            .parent().unwrap()  // build/
            .parent().unwrap()  // release|debug/
            .parent().unwrap(); // target/<triple>/

        // Ищем DLL в сборочных артефактах webview2-com-sys
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
            let dll_dest_debug = target_dir.join("debug").join("WebView2Loader.dll");
            let _ = std::fs::copy(src, &dll_dest);
            let _ = std::fs::copy(src, &dll_dest_debug);
        }
    }
}
