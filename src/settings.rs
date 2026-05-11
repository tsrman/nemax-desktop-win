use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{error, info, warn};

use crate::ua_gen::generate_user_agent;
use crate::exe_dir;

const FILE_SETTINGS_REL: &str = "storage/settings.json";

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub user_agent: String,
    pub allow_service_workers: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub auto_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            user_agent: generate_user_agent(),
            allow_service_workers: false,
            close_to_tray: true,
            start_minimized: false,
            auto_start: false,
        }
    }
}

impl Settings {
    /// Загрузка настроек. При ошибке используются значения по умолчанию.
    pub fn load() -> Self {
        let path = exe_dir().join(FILE_SETTINGS_REL);

        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Settings>(&content) {
                    Ok(s) => {
                        info!("Settings loaded from '{}'", path.display());
                        return s;
                    }
                    Err(e) => warn!("Settings parse error: {}, using defaults", e),
                },
                Err(e) => error!("Cannot read settings: {}, using defaults", e),
            }
        } else {
            info!("No settings file found, creating defaults");
        }

        let s = Settings::default();
        s.save();
        s
    }

    /// Сохранение на диск.
    pub fn save(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&exe_dir().join(FILE_SETTINGS_REL), json) {
                    error!("Failed to save settings: {}", e);
                } else {
                    info!("Settings saved to '{}'", exe_dir().join(FILE_SETTINGS_REL).display());
                }
            }
            Err(e) => error!("Settings serialization failed: {}", e),
        }
    }

    /// Обновление User-Agent.
    pub fn randomize_ua(&mut self) -> String {
        let ua = generate_user_agent();
        self.user_agent = ua.clone();
        self.save();
        ua
    }

    /// Установка Service Workers.
    pub fn set_service_workers(&mut self, enabled: bool) {
        self.allow_service_workers = enabled;
        self.save();
    }

    /// Установка закрытия в трей.
    #[allow(dead_code)]
    pub fn set_close_to_tray(&mut self, val: bool) {
        self.close_to_tray = val;
        self.save();
    }

    /// Установка автозапуска.
    pub fn set_auto_start(&mut self, val: bool) {
        self.auto_start = val;
        self.save();
    }
}
