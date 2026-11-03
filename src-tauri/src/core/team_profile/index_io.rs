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

use crate::core::config::AppConfig;
use crate::core::google::service_account as google_sa;
use crate::core::google_oauth;
use crate::core::workspace_cfg::WorkspaceConfig;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{
    team_id_from_spreadsheet, TeamIndexEntry, TeamsIndex, INDEX_NAME, LEGACY_FILES, TEAMS_DIR,
};

pub(super) fn root() -> PathBuf {
    AppConfig::config_dir()
}

pub(super) fn index_path() -> PathBuf {
    root().join(INDEX_NAME)
}

pub(super) fn teams_root() -> PathBuf {
    root().join(TEAMS_DIR)
}

pub(super) fn team_dir(id: &str) -> PathBuf {
    teams_root().join(id.trim())
}

pub(super) fn load_index_raw() -> TeamsIndex {
    let Ok(raw) = fs::read_to_string(index_path()) else {
        return TeamsIndex::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub(super) fn save_index(idx: &TeamsIndex) -> Result<(), String> {
    let dir = root();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(idx).map_err(|e| e.to_string())?;
    fs::write(index_path(), raw).map_err(|e| e.to_string())
}

/// Если в корне AppData ещё лежат старые flat-файлы — перенести в первый `teams/{id}/`.
pub(super) fn ensure_migrated() {
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

pub(super) fn invalidate_runtime_caches() {
    google_oauth::invalidate_token_mem();
    google_sa::clear_token_cache();
    crate::core::access::clear_presets_cache();
    crate::core::access::invalidate_users_ui_cache();
    crate::core::oauth_config::clear_check_google_cap();
    crate::templates::store::invalidate_cache();
    crate::calculator::article_index::clear();
}

pub(super) fn move_profile_files(from: &Path, to: &Path) {
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

pub(super) fn write_file(path: &Path, raw: &str) -> Result<(), String> {
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

pub(super) fn upsert_entry(idx: &mut TeamsIndex, entry: TeamIndexEntry) {
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
