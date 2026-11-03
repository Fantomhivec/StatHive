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

//! Register admin, join/rename workspace, team invite codes.
use super::common::require_setup_or_main_admin;
use crate::app_state::AppState;
use crate::auth::UserInfo;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
pub struct RegisterAdminArgs {
    pub login: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub workspace_title: String,
}

#[tauri::command]
pub async fn auth_register_admin(
    state: State<'_, Arc<AppState>>,
    args: RegisterAdminArgs,
) -> Result<UserInfo, String> {
    state
        .auth
        .register_admin(
            &args.login,
            &args.password,
            &args.display_name,
            &args.workspace_title,
        )
        .await
}

#[derive(Deserialize)]
pub struct JoinWorkspaceArgs {
    pub spreadsheet_id: String,
    #[serde(default)]
    pub title: String,
}

#[tauri::command]
pub fn auth_join_workspace(
    state: State<'_, Arc<AppState>>,
    args: JoinWorkspaceArgs,
) -> Result<(), String> {
    require_setup_or_main_admin(&state)?;
    state.auth.join_workspace(&args.spreadsheet_id, &args.title)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceArgs {
    pub title: String,
}

#[tauri::command]
pub async fn auth_rename_workspace(
    state: State<'_, Arc<AppState>>,
    args: RenameWorkspaceArgs,
) -> Result<String, String> {
    state.auth.rename_workspace(&args.title).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTeamInviteArgs {
    pub passphrase: String,
}

#[tauri::command]
pub fn auth_export_team_invite(
    state: State<'_, Arc<AppState>>,
    args: ExportTeamInviteArgs,
) -> Result<String, String> {
    state.auth.export_team_invite_code(&args.passphrase)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTeamInviteArgs {
    pub code: String,
    pub passphrase: String,
}

#[tauri::command]
pub async fn auth_apply_team_invite(
    state: State<'_, Arc<AppState>>,
    args: ApplyTeamInviteArgs,
) -> Result<(), String> {
    // Хаб / logout: без сессии можно добавить другую команду. В сессии — только Admin.
    match state.auth.current_user() {
        Some(u) if u.is_authorized => {
            if !crate::core::access::can_manage_team(&u.access_level) {
                return Err(crate::core::i18n::t("errors.auth.needAdminAuth"));
            }
        }
        None => {}
        _ => return Err(crate::core::i18n::t("errors.auth.notAuthorized")),
    }
    state
        .auth
        .apply_team_invite_code(&args.code, &args.passphrase)
}
