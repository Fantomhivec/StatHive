//! Локальный workspace: id таблицы пользователей (владелец — Admin).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const TITLE_SUFFIX: &str = " - Команда";
const LEGACY_TITLE_SUFFIX: &str = " - Workspace";
const DEFAULT_TEAM_NAME: &str = "StatHive";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub spreadsheet_id: String,
    /// Опционально: название для UI.
    #[serde(default)]
    pub title: String,
    /// Машина вошла кодом команды, а не создавала команду сама.
    #[serde(default)]
    pub joined_by_code: bool,
}

fn strip_title_suffix(t: &str) -> String {
    let lower = t.to_ascii_lowercase();
    for suf in [TITLE_SUFFIX, LEGACY_TITLE_SUFFIX] {
        if lower.ends_with(&suf.to_ascii_lowercase()) && t.len() >= suf.len() {
            let s = t[..t.len() - suf.len()].trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    t.to_string()
}

/// Полный title таблицы/команды: `{name} - Команда`.
pub fn normalize_workspace_title(input: &str) -> String {
    let t = input.trim();
    let base = if t.is_empty() {
        DEFAULT_TEAM_NAME.to_string()
    } else {
        strip_title_suffix(t)
    };
    let base = if base.is_empty() {
        DEFAULT_TEAM_NAME
    } else {
        base.as_str()
    };
    format!("{base}{TITLE_SUFFIX}")
}

fn path() -> PathBuf {
    crate::core::team_profile::active_dir().join("workspace.json")
}

pub fn load() -> WorkspaceConfig {
    let p = path();
    // Plaintext по канону; также читаем ошибочно запечатанный at-rest (upsert invite).
    let Some(raw) = crate::core::secure_store::read_text(&p) else {
        return WorkspaceConfig::default();
    };
    let cfg: WorkspaceConfig = serde_json::from_str(&raw).unwrap_or_default();
    if fs::read_to_string(&p)
        .ok()
        .is_some_and(|t| t.trim_start().starts_with("STATHIVE_ATREST_V1."))
    {
        let _ = save(&cfg);
    }
    cfg
}

pub fn save(cfg: &WorkspaceConfig) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // workspace.json — plaintext (только id/title; см. SECURITY_CREDENTIALS).
    fs::write(p, raw).map_err(|e| e.to_string())
}

pub fn spreadsheet_id() -> Option<String> {
    let id = load().spreadsheet_id.trim().to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

pub fn is_configured() -> bool {
    spreadsheet_id().is_some()
}

pub fn set_spreadsheet(id: &str, title: &str) -> Result<(), String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("Укажите ID таблицы workspace".into());
    }
    let mut cfg = load();
    cfg.spreadsheet_id = id.to_string();
    cfg.title = title.trim().to_string();
    save(&cfg)
}

pub fn set_joined_by_code(joined: bool) -> Result<(), String> {
    let mut cfg = load();
    cfg.joined_by_code = joined;
    save(&cfg)
}
