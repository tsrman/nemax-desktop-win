use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

use crate::ua_gen::generate_user_agent;

const FILE_SETTINGS: &str = "storage/settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub user_agent: String,
    pub allow_service_workers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            user_agent: generate_user_agent(),
            allow_service_workers: false,
        }
    }
}

impl Settings {
    /// Загружает настройки из файла. Если файл отсутствует или повреждён — создаёт дефолтные.
    pub fn load() -> Self {
        let path = Path::new(FILE_SETTINGS);

        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<Settings>(&content) {
                    Ok(s) => {
                        info!("Settings loaded from '{}'", FILE_SETTINGS);
                        return s;
                    }
                    Err(e) => warn!("Settings parse error: {} — using defaults", e),
                },
                Err(e) => error!("Cannot read settings: {} — using defaults", e),
            }
        } else {
            info!("No settings file found, creating defaults");
        }

        let s = Settings::default();
        s.save();
        s
    }

    /// Сохраняет настройки на диск.
    pub fn save(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(FILE_SETTINGS, json) {
                    error!("Failed to save settings: {}", e);
                } else {
                    info!("Settings saved to '{}'", FILE_SETTINGS);
                }
            }
            Err(e) => error!("Settings serialization failed: {}", e),
        }
    }

    /// Генерирует и сохраняет новый UA. Возвращает новое значение.
    pub fn randomize_ua(&mut self) -> String {
        let ua = generate_user_agent();
        self.user_agent = ua.clone();
        self.save();
        ua
    }

    /// Устанавливает флаг Service Workers и сохраняет.
    pub fn set_service_workers(&mut self, enabled: bool) {
        self.allow_service_workers = enabled;
        self.save();
    }
}