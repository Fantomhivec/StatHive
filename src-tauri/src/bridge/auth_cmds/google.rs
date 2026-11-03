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

//! OAuth client / service account / Google connect / team OAuth sheet.
use super::common::{require_manage_team, require_oauth_client_edit};
use crate::app_state::AppState;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
pub struct SaveOauthClientArgs {
    pub json: String,
}

#[tauri::command]
pub fn auth_save_oauth_client(
    state: State<'_, Arc<AppState>>,
    args: SaveOauthClientArgs,
) -> Result<(), String> {
    require_oauth_client_edit(&state)?;
    state.auth.save_oauth_client(&args.json)
}

#[derive(Deserialize)]
pub struct SaveServiceAccountArgs {
    pub json: String,
}

#[tauri::command]
pub async fn auth_save_service_account(
    state: State<'_, Arc<AppState>>,
    args: SaveServiceAccountArgs,
) -> Result<String, String> {
    state.auth.save_service_account(&args.json).await
}

#[tauri::command]
pub async fn auth_create_workspace_service_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.auth.create_workspace_service_account().await
}

#[tauri::command]
pub async fn auth_replace_workspace_service_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.auth.replace_workspace_service_account().await
}

#[tauri::command]
pub async fn auth_connect_google(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    require_oauth_client_edit(&state)?;
    state.auth.connect_google().await
}

#[tauri::command]
pub async fn auth_switch_google_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    require_oauth_client_edit(&state)?;
    state.auth.switch_google_account().await
}

#[tauri::command]
pub async fn auth_team_oauth_get(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::auth::team_oauth_sheet::TeamOauthState, String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::get_state().await
}

#[tauri::command]
pub async fn auth_team_oauth_publish(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::publish_from_local().await?;
    crate::auth::team_oauth_sheet::log_publish_ok(&state.log);
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamOauthSetGrantsArgs {
    pub allowed_logins: Vec<String>,
}

#[tauri::command]
pub async fn auth_team_oauth_set_grants(
    state: State<'_, Arc<AppState>>,
    args: TeamOauthSetGrantsArgs,
) -> Result<(), String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::set_grants(&args.allowed_logins).await
}
