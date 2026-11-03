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

//! Shared guards, version, process logs.
use crate::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn app_version() -> String {
    crate::core::app_version::current().into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsGetArgs {
    #[serde(default)]
    pub known_revision: u64,
}

#[tauri::command]
pub fn logs_get(
    state: State<'_, Arc<AppState>>,
    args: Option<LogsGetArgs>,
) -> Result<serde_json::Value, String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    let known = args.map(|a| a.known_revision).unwrap_or(0);
    let (lines, revision) = state.log.snapshot_if_changed(known);
    Ok(serde_json::json!({
        "revision": revision,
        "unchanged": lines.is_none(),
        "lines": lines.unwrap_or_default(),
    }))
}

#[tauri::command]
pub fn logs_clear(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    state.log.clear();
    Ok(())
}

/// Первый setup без workspace, иначе только main Admin.
/// Не keysCheck: иначе участник после logout перезапишет OAuth-клиент машины.
pub(super) fn require_setup_or_main_admin(state: &AppState) -> Result<(), String> {
    let workspace_ready = crate::core::workspace_cfg::is_configured();
    match state.auth.current_user() {
        None => {
            if workspace_ready {
                Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
            } else {
                Ok(())
            }
        }
        Some(u) if u.is_authorized => {
            if crate::core::access::is_admin_role(&u.access_level) {
                Ok(())
            } else {
                Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
            }
        }
        _ => Err(crate::core::i18n::t("errors.auth.notAuthorized")),
    }
}

/// Desktop OAuth JSON: setup без workspace, иначе только main Admin (как UI).
/// Не keysCheck: участник не перезаписывает командный client на shared PC.
pub(super) fn require_oauth_client_edit(state: &AppState) -> Result<(), String> {
    require_setup_or_main_admin(state)
}

pub(super) fn require_manage_team(state: &AppState) -> Result<crate::auth::UserInfo, String> {
    let u = state
        .auth
        .current_user()
        .filter(|u| u.is_authorized)
        .ok_or_else(|| crate::core::i18n::t("errors.auth.notAuthorized"))?;
    if crate::core::access::can_manage_team(&u.access_level) {
        Ok(u)
    } else {
        Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
    }
}
