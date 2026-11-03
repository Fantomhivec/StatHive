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

//! Invite / list / delete users and users dashboard.
use crate::app_state::AppState;
use crate::auth::{TeamUserRow, UsersDashboard};
use crate::bridge::access_cmds::PresetDto;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteUserArgs {
    pub login: String,
    pub password: String,
    pub display_name: String,
    pub google_email: String,
    /// Обязательный id пресета прав (лист «Пресеты»).
    pub preset_id: String,
    /// Id конфигурации CalcTeam (пусто → default).
    #[serde(default)]
    pub calc_config_id: String,
}

#[tauri::command]
pub async fn auth_invite_user(
    state: State<'_, Arc<AppState>>,
    args: InviteUserArgs,
) -> Result<crate::auth::TeamUserRow, String> {
    let login = args.login.clone();
    state.auth
        .invite_user(
            &args.login,
            &args.password,
            &args.display_name,
            &args.google_email,
            &args.preset_id,
            &args.calc_config_id,
        )
        .await
        .map_err(|e| {
            state.log.error(crate::core::i18n::tf(
                "log.auth.inviteFailed",
                &[("login", &login), ("error", &e)],
            ));
            e
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserCalcConfigArgs {
    pub row_index: u32,
    pub calc_config_id: String,
}

#[tauri::command]
pub async fn auth_set_user_calc_config(
    state: State<'_, Arc<AppState>>,
    args: SetUserCalcConfigArgs,
) -> Result<(), String> {
    let gen = crate::auth::team::next_assign_generation(args.row_index);
    let row = args.row_index;
    state
        .auth
        .set_user_calc_config(row, &args.calc_config_id, gen)
        .await
        .map_err(|e| {
            state.log.error(crate::core::i18n::tf(
                "log.auth.inviteFailed",
                &[("login", &row.to_string()), ("error", &e)],
            ));
            e
        })
}

#[tauri::command]
pub async fn auth_list_team(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<TeamUserRow>, String> {
    state.auth.list_team().await
}

#[derive(serde::Deserialize)]
pub struct UsersDashboardArgs {
    #[serde(default)]
    pub force: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersDashboardDto {
    pub team: Vec<TeamUserRow>,
    pub presets: Vec<PresetDto>,
    pub workspace_id: Option<String>,
    pub workspace_title: String,
    pub service_account_configured: bool,
    pub service_account_email: Option<String>,
}

impl From<UsersDashboard> for UsersDashboardDto {
    fn from(d: UsersDashboard) -> Self {
        Self {
            team: d.team,
            presets: d.presets.into_iter().map(PresetDto::from).collect(),
            workspace_id: d.workspace_id,
            workspace_title: d.workspace_title,
            service_account_configured: d.service_account_configured,
            service_account_email: d.service_account_email,
        }
    }
}

#[tauri::command]
pub async fn auth_users_dashboard(
    state: State<'_, Arc<AppState>>,
    args: UsersDashboardArgs,
) -> Result<UsersDashboardDto, String> {
    let dash = state.auth.users_dashboard(args.force).await?;
    Ok(UsersDashboardDto::from(dash))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserArgs {
    pub row_index: u32,
}

#[tauri::command]
pub async fn auth_delete_user(
    state: State<'_, Arc<AppState>>,
    args: DeleteUserArgs,
) -> Result<(), String> {
    state.auth.delete_user(args.row_index).await
}
