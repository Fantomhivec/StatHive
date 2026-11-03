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
use crate::core::spreadsheet_id;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
pub struct NormalizeSpreadsheetArgs {
    pub value: String,
}

#[tauri::command]
pub fn spreadsheet_normalize(args: NormalizeSpreadsheetArgs) -> serde_json::Value {
    let (id, changed) = spreadsheet_id::normalize(&args.value);
    serde_json::json!({ "spreadsheet_id": id, "changed": changed })
}

#[tauri::command]
pub async fn tabs_layout_get(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let user = state
        .auth
        .current_user()
        .ok_or_else(|| t("errors.auth.notAuthorized"))?;
    state.auth.ensure_presets_loaded().await;
    crate::auth::team_oauth_sheet::auto_pull_if_needed(
        &user.access_level,
        &user.login,
        &state.log,
    )
    .await;
    let can_check = crate::auth::team_oauth_sheet::refresh_can_check_google(
        &user.access_level,
        &user.login,
    )
    .await;
    let any_ok =
        crate::templates::access_gate::any_member_ok_in_scope(&state, &user.access_level);
    let oauth = access::effective_has_oauth(&user.access_level, user.has_oauth);
    Ok(crate::core::tabs_layout::get_layout(
        &user.login,
        &user.access_level,
        oauth,
        any_ok,
        can_check,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabsLayoutSaveArgs {
    #[serde(default)]
    pub items: Vec<crate::core::tabs_layout::TabLayoutItem>,
    pub calculator_mode: Option<bool>,
}

#[tauri::command]
pub fn tabs_layout_save(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    args: TabsLayoutSaveArgs,
) -> Result<serde_json::Value, String> {
    use tauri::Emitter;
    let user = state
        .auth
        .current_user()
        .ok_or_else(|| t("errors.auth.notAuthorized"))?;
    let any_ok =
        crate::templates::access_gate::any_member_ok_in_scope(&state, &user.access_level);
    let oauth = access::effective_has_oauth(&user.access_level, user.has_oauth);
    let result = crate::core::tabs_layout::save_layout(
        &user.login,
        &user.access_level,
        args.items,
        args.calculator_mode,
        oauth,
        any_ok,
    )?;
    let _ = app.emit("tabsLayout.changed", &result);
    Ok(result)
}
