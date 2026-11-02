//! Каталог конфигураций калькулятора — лист `CalcTeam` в workspace (SA).

use crate::calculator::profile::CalculatorProfile;
use crate::core::config::AppConfig;
use crate::core::i18n::t;
use crate::core::spreadsheet_id;
use crate::core::workspace_cfg;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;


pub const CALC_TEAM_SHEET: &str = "CalcTeam";

/// Лимит ячейки Sheets ~50k; берём запас.
pub(crate) const CHUNK_CHARS: usize = 40_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfigEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub revision: String,
    pub profile: CalculatorProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfigCatalog {
    #[serde(default)]
    pub default_id: String,
    #[serde(default)]
    pub configs: Vec<TeamConfigEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfigListItem {
    pub id: String,
    pub name: String,
    pub revision: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppliedStamp {
    #[serde(default)]
    pub config_id: String,
    #[serde(default)]
    pub revision: String,
}

pub(crate) fn spreadsheet_id_ws() -> Result<String, String> {
    let ws = workspace_cfg::load();
    let sid = ws.spreadsheet_id.trim().to_string();
    if sid.is_empty() {
        return Err(t("errors.auth.workspaceNotConfigured"));
    }
    Ok(sid)
}

pub(crate) fn stamp_path() -> PathBuf {
    AppConfig::config_dir().join("calc_team_applied.json")
}

pub fn load_stamp() -> AppliedStamp {
    let p = stamp_path();
    fs::read_to_string(&p)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_stamp(config_id: &str, revision: &str) -> Result<(), String> {
    let dir = AppConfig::config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(&AppliedStamp {
        config_id: config_id.to_string(),
        revision: revision.to_string(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(stamp_path(), raw).map_err(|e| e.to_string())
}

pub(crate) fn chunk_json(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut rest = raw;
    while !rest.is_empty() {
        let mut end = rest.len().min(CHUNK_CHARS);
        if end < rest.len() {
            while end > 0 && !rest.is_char_boundary(end) {
                end -= 1;
            }
            if end == 0 {
                end = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            }
        }
        out.push(rest[..end].to_string());
        rest = &rest[end..];
    }
    out
}

pub(crate) fn new_revision() -> String {
    Uuid::new_v4().simple().to_string()
}

pub(crate) fn new_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Id таблиц из ссылок профиля (без workspace).
pub fn table_ids_from_profile(profile: &CalculatorProfile) -> HashSet<String> {
    let mut set = HashSet::new();
    for url in profile.table_links.values() {
        if let Some(id) = spreadsheet_id::try_extract(url) {
            let id = id.trim().to_string();
            if !id.is_empty() {
                set.insert(id);
            }
        }
    }
    set
}

/// Для Drive reader по Gmail: без флага `tables` — пустые tableLinks / bindings (revoke).
pub fn profile_for_user_drive(
    access_level: &str,
    profile: &CalculatorProfile,
) -> CalculatorProfile {
    use crate::core::access::{self, CALC_FLAG_TABLES};
    if access::is_admin_role(access_level) || access::can_calc(access_level, CALC_FLAG_TABLES) {
        return profile.clone();
    }
    let mut p = profile.clone();
    p.table_links.clear();
    p.column_bindings.clear();
    p.template_ids_by_table_key.clear();
    p
}

