/// Модуль управления автозапуском.
/// - Windows: через реестр
/// - Linux: через .desktop файл в ~/.config/autostart/
/// - macOS: no-op заглушка

#[cfg(target_os = "windows")]
mod imp {
    use std::error::Error;
    use winreg::enums::*;
    use winreg::RegKey;

    const REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const REG_NAME: &str = "neMAX";

    /// Включает или выключает автозапуск приложения.
    pub fn set_auto_start(enabled: bool) -> Result<(), Box<dyn Error>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(REG_PATH)?;

        if enabled {
            let exe_path = std::env::current_exe()?;
            let value = format!("\"{}\" --minimized", exe_path.display());
            key.set_value(REG_NAME, &value)?;
        } else {
            // Игнорируем ошибку если значения нет
            let _ = key.delete_value(REG_NAME);
        }

        Ok(())
    }

    /// Проверяет, включён ли автозапуск.
    #[allow(dead_code)]
    pub fn is_auto_start_enabled() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(REG_PATH) {
            key.get_value::<String, _>(REG_NAME).is_ok()
        } else {
            false
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::error::Error;
    use std::fs;
    use std::path::PathBuf;

    const DESKTOP_FILE_NAME: &str = "nemax.desktop";

    fn get_autostart_dir() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".config"))
            })
            .map(|p| p.join("autostart"))
    }

    fn get_desktop_file_path() -> Option<PathBuf> {
        get_autostart_dir().map(|dir| dir.join(DESKTOP_FILE_NAME))
    }

    fn generate_desktop_file(exe_path: &str) -> String {
        format!(
            r#"[Desktop Entry]
Type=Application
Name=neMAX
Comment=Lightweight max.ru client
Exec="{}" --minimized
Icon=nemax
Terminal=false
Categories=Network;
StartupNotify=false
"#,
            exe_path
        )
    }

    /// Включает или выключает автозапуск приложения.
    pub fn set_auto_start(enabled: bool) -> Result<(), Box<dyn Error>> {
        if let Some(desktop_path) = get_desktop_file_path() {
            if enabled {
                let exe_path = std::env::current_exe()?
                    .to_string_lossy()
                    .to_string();
                
                // Создаём директорию autostart если её нет
                if let Some(parent) = desktop_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let content = generate_desktop_file(&exe_path);
                fs::write(&desktop_path, content)?;
                
                // Делаем файл исполняемым (опционально, но хорошая практика)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&desktop_path)?.permissions();
                    perms.set_mode(perms.mode() | 0o755);
                    fs::set_permissions(&desktop_path, perms)?;
                }
            } else {
                // Игнорируем ошибку если файла нет
                let _ = fs::remove_file(&desktop_path);
            }
        }

        Ok(())
    }

    /// Проверяет, включён ли автозапуск.
    #[allow(dead_code)]
    pub fn is_auto_start_enabled() -> bool {
        if let Some(desktop_path) = get_desktop_file_path() {
            desktop_path.exists()
        } else {
            false
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod imp {
    use std::error::Error;

    pub fn set_auto_start(_enabled: bool) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_auto_start_enabled() -> bool {
        false
    }
}

pub use imp::*;
