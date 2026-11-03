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

//! Local team hub: list / switch / create / remove.
use crate::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamIdArgs {
    pub id: String,
}

#[tauri::command]
pub fn auth_list_teams() -> Vec<crate::core::team_profile::TeamIndexEntry> {
    crate::core::team_profile::list_teams()
}

#[tauri::command]
pub fn auth_begin_new_team(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    // Хаб «Настроить новый OAuth» до логина: пустой слот new-*, без сессии Admin.
    state.auth.begin_new_team()
}

#[tauri::command]
pub fn auth_set_active_team(
    state: State<'_, Arc<AppState>>,
    args: TeamIdArgs,
) -> Result<(), String> {
    state.auth.set_active_team(&args.id)
}

#[tauri::command]
pub fn auth_remove_local_team(
    state: State<'_, Arc<AppState>>,
    args: TeamIdArgs,
) -> Result<(), String> {
    state.auth.remove_local_team(&args.id)
}

#[tauri::command]
pub async fn auth_switch_team(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    args: TeamIdArgs,
) -> Result<crate::auth::SwitchTeamResult, String> {
    use tauri::Emitter;
    let result = state.auth.switch_team(&args.id).await?;
    if let Some(ref user) = result.user {
        state.auth.ensure_presets_loaded().await;
        let any_ok =
            crate::templates::access_gate::any_member_ok_in_scope(&state, &user.access_level);
        let oauth = crate::core::access::effective_has_oauth(&user.access_level, user.has_oauth);
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
    Ok(result)
}
