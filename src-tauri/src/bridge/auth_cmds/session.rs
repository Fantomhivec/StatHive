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

//! Login / logout / refresh / password / session status.
use crate::app_state::AppState;
use crate::auth::{AuthSetupStatus, UserInfo};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn auth_status(state: State<'_, Arc<AppState>>) -> Option<UserInfo> {
    state.auth.ensure_activity_timer();
    state.auth.current_user()
}

#[tauri::command]
pub fn auth_setup_status(state: State<'_, Arc<AppState>>) -> AuthSetupStatus {
    state.auth.setup_status()
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, Arc<AppState>>,
    login: String,
    password: String,
    google_email: Option<String>,
) -> Result<UserInfo, String> {
    state
        .auth
        .login(&login, &password, google_email.as_deref().unwrap_or(""))
        .await
}

#[tauri::command]
pub async fn auth_logout(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.auth.logout().await;
    Ok(())
}

#[tauri::command]
pub async fn auth_refresh_profile(
    state: State<'_, Arc<AppState>>,
) -> Result<UserInfo, String> {
    state.auth.refresh_profile().await
}

#[derive(Deserialize)]
pub struct ChangePasswordArgs {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[tauri::command]
pub async fn auth_change_password(
    state: State<'_, Arc<AppState>>,
    args: ChangePasswordArgs,
) -> Result<serde_json::Value, String> {
    match state
        .auth
        .change_password(
            &args.current_password,
            &args.new_password,
            &args.confirm_password,
        )
        .await
    {
        Ok(()) => Ok(serde_json::json!({ "success": true })),
        Err(e) => Ok(serde_json::json!({ "success": false, "error": e })),
    }
}

#[tauri::command]
pub async fn auth_connect_member_oauth(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<UserInfo, String> {
    use tauri::Emitter;
    let user = state.auth.connect_member_oauth().await?;
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
    Ok(user)
}
