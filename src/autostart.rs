/// Модуль управления автозапуском через реестр Windows.
/// На других платформах — no-op заглушки.

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

#[cfg(not(target_os = "windows"))]
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
