use crate::core::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Опциональный путь к `.env` (кнопка «Выбрать .env»). Пусто → рядом с exe.
    #[serde(default)]
    pub env_file_path: String,
    /// Куда писать результат скриптов (Google Sheets).
    pub result_spreadsheet_id: String,
    pub result_sheet_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            env_file_path: String::new(),
            result_spreadsheet_id: String::new(),
            result_sheet_name: String::new(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        paths::app_config_dir()
    }

    pub fn path() -> PathBuf {
        Self::config_dir().join("app.config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let cfg = if let Ok(raw) = fs::read_to_string(&path) {
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Self::default()
        };
        paths::purge_stray_env_copies();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::path(), raw).map_err(|e| e.to_string())
    }

    /// Только `env_file_path` из файла конфига (без save / purge).
    pub fn peek_env_file_path() -> String {
        let path = Self::path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<AppConfig>(&raw).ok())
            .map(|c| c.env_file_path)
            .unwrap_or_default()
    }
}
