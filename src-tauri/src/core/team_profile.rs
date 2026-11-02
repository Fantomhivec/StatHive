//! Несколько команд на одном ПК: `teams_index.json` + `teams/{id}/…`.

use crate::core::config::AppConfig;
use crate::core::google::service_account as google_sa;
use crate::core::google_oauth;
use crate::core::workspace_cfg::{self, WorkspaceConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const INDEX_NAME: &str = "teams_index.json";
const TEAMS_DIR: &str = "teams";
const LEGACY_FILES: &[&str] = &[
    "workspace.json",
    "oauth_client.json",
    "service_account.json",
    "google_oauth_token.json",
    "session.json",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamIndexEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub joined_by_code: bool,
    /// `Admin` | `Member` — после успешного login.
    #[serde(default)]
    pub last_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamsIndex {
    #[serde(default)]
    pub active_id: String,
    #[serde(default)]
    pub teams: Vec<TeamIndexEntry>,
}

fn root() -> PathBuf {
    AppConfig::config_dir()
}

fn index_path() -> PathBuf {
    root().join(INDEX_NAME)
}

fn teams_root() -> PathBuf {
    root().join(TEAMS_DIR)
}

pub fn team_id_from_spreadsheet(spreadsheet_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(spreadsheet_id.trim().as_bytes());
    let dig = h.finalize();
    hex::encode(&dig[..8])
}

fn load_index_raw() -> TeamsIndex {
    let Ok(raw) = fs::read_to_string(index_path()) else {
        return TeamsIndex::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_index(idx: &TeamsIndex) -> Result<(), String> {
    let dir = root();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(idx).map_err(|e| e.to_string())?;
    fs::write(index_path(), raw).map_err(|e| e.to_string())
}

/// Если в корне AppData ещё лежат старые flat-файлы — перенести в первый `teams/{id}/`.
pub fn ensure_migrated() {
    let _ = migrate_legacy();
}

fn migrate_legacy() -> Result<(), String> {
    let mut idx = load_index_raw();
    if !idx.teams.is_empty() {
        if idx.active_id.trim().is_empty() || !team_dir(&idx.active_id).is_dir() {
            if let Some(first) = idx.teams.first() {
                idx.active_id = first.id.clone();
                let _ = save_index(&idx);
            }
        }
        return Ok(());
    }

    let legacy_ws = root().join("workspace.json");
    if !legacy_ws.is_file() {
        if !index_path().is_file() {
            let _ = save_index(&TeamsIndex::default());
        }
        return Ok(());
    }

    let ws: WorkspaceConfig = fs::read_to_string(&legacy_ws)
        .ok()
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default();
    let sid = ws.spreadsheet_id.trim();
    if sid.is_empty() {
        return Ok(());
    }
    let id = team_id_from_spreadsheet(sid);
    let dest = team_dir(&id);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    for name in LEGACY_FILES {
        let from = root().join(name);
        if from.is_file() {
            let to = dest.join(name);
            if !to.exists() {
                let _ = fs::rename(&from, &to).or_else(|_| {
                    fs::copy(&from, &to).map(|_| ()).and_then(|_| fs::remove_file(&from))
                });
            }
        }
    }
    idx.teams.push(TeamIndexEntry {
        id: id.clone(),
        title: ws.title.clone(),
        joined_by_code: ws.joined_by_code,
        last_role: if ws.joined_by_code {
            "Member".into()
        } else {
            "Admin".into()
        },
    });
    idx.active_id = id;
    save_index(&idx)?;
    Ok(())
}

pub fn team_dir(id: &str) -> PathBuf {
    teams_root().join(id.trim())
}

/// Каталог активного профиля команды (после migrate).
pub fn active_dir() -> PathBuf {
    ensure_migrated();
    let idx = load_index_raw();
    let id = idx.active_id.trim();
    if !id.is_empty() {
        let d = team_dir(id);
        if d.is_dir() {
            return d;
        }
    }
    // Fallback: root (до первой команды / oauth staging).
    root()
}

pub fn active_id() -> Option<String> {
    ensure_migrated();
    let id = load_index_raw().active_id.trim().to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

pub fn list_teams() -> Vec<TeamIndexEntry> {
    ensure_migrated();
    load_index_raw().teams
}

pub fn set_active(id: &str) -> Result<(), String> {
    ensure_migrated();
    let id = id.trim();
    if id.is_empty() {
        return Err("Пустой id команды".into());
    }
    let mut idx = load_index_raw();
    if !idx.teams.iter().any(|t| t.id == id) {
        return Err("Команда не найдена".into());
    }
    let dest = team_dir(id);
    if !dest.is_dir() {
        return Err("Нет каталога профиля команды".into());
    }
    idx.active_id = id.to_string();
    save_index(&idx)?;
    invalidate_runtime_caches();
    Ok(())
}

fn invalidate_runtime_caches() {
    google_oauth::invalidate_token_mem();
    google_sa::clear_token_cache();
    crate::core::access::clear_presets_cache();
    crate::core::oauth_config::clear_check_google_cap();
}

/// Новый пустой слот под Admin OAuth (пока нет spreadsheet id).
pub fn begin_new_team() -> Result<String, String> {
    ensure_migrated();
    let id = format!("new-{}", Uuid::new_v4().simple());
    let dest = team_dir(&id);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let mut idx = load_index_raw();
    idx.teams.retain(|t| !t.id.starts_with("new-") || t.id == id);
    // Удалить старые незавершённые new-* каталоги.
    if let Ok(rd) = fs::read_dir(teams_root()) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("new-") && name != id {
                let _ = fs::remove_dir_all(e.path());
                idx.teams.retain(|t| t.id != name);
            }
        }
    }
    if !idx.teams.iter().any(|t| t.id == id) {
        idx.teams.push(TeamIndexEntry {
            id: id.clone(),
            title: String::new(),
            joined_by_code: false,
            last_role: "Admin".into(),
        });
    }
    idx.active_id = id.clone();
    save_index(&idx)?;
    invalidate_runtime_caches();
    Ok(id)
}

fn move_profile_files(from: &Path, to: &Path) {
    let _ = fs::create_dir_all(to);
    for name in LEGACY_FILES {
        let src = from.join(name);
        if !src.is_file() {
            continue;
        }
        let dest = to.join(name);
        let _ = fs::rename(&src, &dest).or_else(|_| {
            fs::copy(&src, &dest)
                .map(|_| ())
                .and_then(|_| fs::remove_file(&src))
        });
    }
}

/// После create spreadsheet: `new-*` или root → `teams/{hash}` + index.
pub fn finalize_new_team(spreadsheet_id: &str, title: &str) -> Result<String, String> {
    ensure_migrated();
    let final_id = team_id_from_spreadsheet(spreadsheet_id);
    let mut idx = load_index_raw();
    let active = idx.active_id.trim().to_string();
    let to = team_dir(&final_id);

    if active != final_id {
        let from = team_dir(&active);
        if !active.is_empty() && from.is_dir() {
            if to.is_dir() {
                move_profile_files(&from, &to);
                let _ = fs::remove_dir_all(&from);
            } else {
                fs::rename(&from, &to).map_err(|e| e.to_string())?;
            }
            idx.teams.retain(|t| t.id != active);
        } else {
            move_profile_files(&root(), &to);
        }
    } else if !to.is_dir() {
        move_profile_files(&root(), &to);
    }

    upsert_entry(
        &mut idx,
        TeamIndexEntry {
            id: final_id.clone(),
            title: title.trim().to_string(),
            joined_by_code: false,
            last_role: "Admin".into(),
        },
    );
    idx.active_id = final_id.clone();
    save_index(&idx)?;
    invalidate_runtime_caches();
    Ok(final_id)
}

/// Код команды UR3: upsert профиля по spreadsheet id, SA + workspace (OAuth не из кода).
pub fn upsert_from_invite(
    spreadsheet_id: &str,
    title: &str,
    sa_raw: &str,
) -> Result<String, String> {
    ensure_migrated();
    let id = team_id_from_spreadsheet(spreadsheet_id);
    let dest = team_dir(&id);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    write_file(&dest.join("service_account.json"), sa_raw)?;
    let ws = WorkspaceConfig {
        spreadsheet_id: spreadsheet_id.trim().to_string(),
        title: title.trim().to_string(),
        joined_by_code: true,
    };
    let ws_raw = serde_json::to_string_pretty(&ws).map_err(|e| e.to_string())?;
    write_file(&dest.join("workspace.json"), &ws_raw)?;

    let mut idx = load_index_raw();
    let prev_role = idx
        .teams
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.last_role.clone())
        .unwrap_or_else(|| "Member".into());
    upsert_entry(
        &mut idx,
        TeamIndexEntry {
            id: id.clone(),
            title: title.trim().to_string(),
            joined_by_code: true,
            last_role: prev_role,
        },
    );
    idx.active_id = id.clone();
    save_index(&idx)?;
    invalidate_runtime_caches();
    Ok(id)
}

pub fn update_last_role(role: &str) -> Result<(), String> {
    ensure_migrated();
    let mut idx = load_index_raw();
    let active = idx.active_id.clone();
    if let Some(t) = idx.teams.iter_mut().find(|t| t.id == active) {
        t.last_role = role.trim().to_string();
        // sync title from workspace
        let ws = workspace_cfg::load();
        if !ws.title.trim().is_empty() {
            t.title = ws.title.clone();
        }
        t.joined_by_code = ws.joined_by_code;
    }
    save_index(&idx)
}

pub fn update_active_title(title: &str) -> Result<(), String> {
    ensure_migrated();
    let mut idx = load_index_raw();
    let active = idx.active_id.clone();
    if let Some(t) = idx.teams.iter_mut().find(|t| t.id == active) {
        t.title = title.trim().to_string();
    }
    save_index(&idx)
}

pub fn entry(id: &str) -> Option<TeamIndexEntry> {
    ensure_migrated();
    load_index_raw()
        .teams
        .into_iter()
        .find(|t| t.id == id.trim())
}

fn upsert_entry(idx: &mut TeamsIndex, entry: TeamIndexEntry) {
    if let Some(t) = idx.teams.iter_mut().find(|t| t.id == entry.id) {
        t.title = entry.title;
        t.joined_by_code = entry.joined_by_code;
        if !entry.last_role.is_empty() {
            t.last_role = entry.last_role;
        }
    } else {
        idx.teams.push(entry);
    }
}

fn write_file(path: &Path, raw: &str) -> Result<(), String> {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    // workspace + SA + oauth_client + token + session — plaintext (не зависят от keyring).
    if name.eq_ignore_ascii_case("workspace.json")
        || name.eq_ignore_ascii_case("service_account.json")
        || name.eq_ignore_ascii_case("oauth_client.json")
        || name.eq_ignore_ascii_case("google_oauth_token.json")
        || name.eq_ignore_ascii_case("session.json")
    {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        return fs::write(path, raw.trim()).map_err(|e| e.to_string());
    }
    crate::core::secure_store::write_text(path, raw.trim())
}

/// Цель — Admin (для switch: браузер Google).
pub fn target_needs_google(entry: &TeamIndexEntry) -> bool {
    let role = entry.last_role.trim();
    if role.eq_ignore_ascii_case("Admin") {
        return true;
    }
    if role.eq_ignore_ascii_case("Member") {
        return false;
    }
    // Нет lastRole: локально созданная команда → Admin.
    !entry.joined_by_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_stable() {
        let a = team_id_from_spreadsheet("abc");
        let b = team_id_from_spreadsheet("abc");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }
}
