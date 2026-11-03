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

use crate::app_state::AppState;
use crate::core::access;
use crate::core::i18n::t;

pub(super) fn env_path_from_state(state: &AppState) -> Result<std::path::PathBuf, String> {
    let mut cfg = state.config.lock().map_err(|_| "config lock")?;
    // Только *.env рядом с exe (keys.env, cab.env, .env, …).
    let raw = cfg.env_file_path.trim().to_string();
    if !raw.is_empty() {
        if is_appdata_env_path(&raw) {
            cfg.env_file_path.clear();
            let _ = cfg.save();
        } else if let Ok(p) = crate::core::paths::ensure_env_beside_exe(std::path::Path::new(&raw))
        {
            return Ok(p);
        } else {
            cfg.env_file_path.clear();
            let _ = cfg.save();
        }
    }
    Ok(crate::core::paths::env_file_path())
}

fn is_appdata_env_path(raw: &str) -> bool {
    let p = raw.trim();
    if p.is_empty() {
        return false;
    }
    let lower = p.replace('/', "\\").to_ascii_lowercase();
    lower.contains("\\appdata\\")
        && (lower.contains("\\stathive\\") || lower.contains("\\universalrust\\"))
        && lower.ends_with("\\.env")
}

pub(super) fn require_keys_check(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if !access::can_keys_check(&u.access_level) {
        return Err(t("errors.auth.keysCheckDenied"));
    }
    Ok(())
}

/// KeysCheck **или** (OAuth ∧ скрипты) — Google list/check (нужен token).
pub(super) fn require_keys_or_script_oauth(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if access::can_keys_check(&u.access_level) {
        return Ok(());
    }
    let oauth = access::effective_has_oauth(&u.access_level, u.has_oauth);
    let any_member_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &u.access_level);
    if oauth && access::can_use_any_script_ex(&u.access_level, oauth, any_member_ok) {
        return Ok(());
    }
    Err(t("errors.auth.keysCheckDenied"))
}

/// KeysCheck **или** любой разрешённый скрипт (в т.ч. Excel-only / memberOk) — имена кабинетов без секретов.
pub(super) fn require_keys_or_any_script(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if access::can_keys_check(&u.access_level) {
        return Ok(());
    }
    let oauth = access::effective_has_oauth(&u.access_level, u.has_oauth);
    let any_member_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &u.access_level);
    if access::can_use_any_script_ex(&u.access_level, oauth, any_member_ok) {
        return Ok(());
    }
    Err(t("errors.auth.keysCheckDenied"))
}

pub(super) fn emit_tabs_layout(app: &tauri::AppHandle, state: &AppState) {
    use tauri::Emitter;
    let Some(user) = state.auth.current_user().filter(|u| u.is_authorized) else {
        return;
    };
    let any_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &user.access_level);
    let oauth = access::effective_has_oauth(&user.access_level, user.has_oauth);
    let can_check =
        crate::core::oauth_config::can_check_google(&user.access_level, &user.login);
    let layout = crate::core::tabs_layout::get_layout(
        &user.login,
        &user.access_level,
        oauth,
        any_ok,
        can_check,
    );
    let _ = app.emit("tabsLayout.changed", &layout);
}
