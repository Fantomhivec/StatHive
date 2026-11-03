// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::core::workspace_cfg::{self, WorkspaceConfig};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use super::index_io::{
    invalidate_runtime_caches, load_index_raw, move_profile_files, root, save_index, teams_root,
    upsert_entry, write_file,
};
use super::types::{team_id_from_spreadsheet, TeamIndexEntry};

pub fn ensure_migrated() {
    super::index_io::ensure_migrated();
}

pub fn team_dir(id: &str) -> PathBuf {
    super::index_io::team_dir(id)
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

/// Файл ещё в корне AppData (старый общий calc): в единственную команду — перенести;
/// при нескольких командах — удалить, чтобы не занести чужой профиль.
pub fn absorb_legacy_root_file(name: &str) {
    ensure_migrated();
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let root_dir = root();
    let legacy = root_dir.join(name);
    if !legacy.is_file() {
        return;
    }
    let active = active_dir();
    if active == root_dir {
        return;
    }
    let dest = active.join(name);
    if dest.is_file() {
        let _ = fs::remove_file(&legacy);
        return;
    }
    if load_index_raw().teams.len() == 1 {
        let _ = fs::create_dir_all(&active);
        let _ = fs::rename(&legacy, &dest).or_else(|_| {
            fs::copy(&legacy, &dest)
                .map(|_| ())
                .and_then(|_| fs::remove_file(&legacy))
        });
    } else {
        let _ = fs::remove_file(&legacy);
    }
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

/// Удалить локальный профиль команды (OAuth, token, session, workspace, SA) + индекс.
/// Таблицу в Google Drive не трогает.
pub fn remove_team(id: &str) -> Result<(), String> {
    ensure_migrated();
    let id = id.trim();
    if id.is_empty() {
        return Err("Пустой id команды".into());
    }
    let mut idx = load_index_raw();
    if !idx.teams.iter().any(|t| t.id == id) {
        return Err("Команда не найдена".into());
    }
    let dir = team_dir(id);
    if dir.is_dir() {
        let _ = fs::remove_dir_all(&dir);
    }
    idx.teams.retain(|t| t.id != id);
    if idx.active_id.trim() == id {
        idx.active_id = idx
            .teams
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_default();
    }
    save_index(&idx)?;
    invalidate_runtime_caches();
    Ok(())
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
