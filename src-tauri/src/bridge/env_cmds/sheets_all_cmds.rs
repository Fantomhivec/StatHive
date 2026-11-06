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

//! Вкладка «Все таблицы»: list / delete / permissions.

use crate::app_state::AppState;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::t;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tauri::State;

use super::guards::require_all_tables_oauth;

fn normalize_spreadsheet_id(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if let Some(cap) = s.find("/spreadsheets/d/") {
        let rest = &s[cap + "/spreadsheets/d/".len()..];
        let id = rest.split('/').next().unwrap_or("").trim();
        if !id.is_empty() {
            return id.to_string();
        }
    }
    s.to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetsListOwnedArgs {
    #[serde(default)]
    pub shared_only: bool,
}

#[tauri::command]
pub async fn sheets_list_owned(
    state: State<'_, Arc<AppState>>,
    args: SheetsListOwnedArgs,
) -> Result<serde_json::Value, String> {
    require_all_tables_oauth(&state)?;
    let client = GoogleSheetsClient::from_user_oauth().await?;
    let files = client.list_accessible_spreadsheets().await?;
    let items = GoogleSheetsClient::map_all_tables_rows(files, args.shared_only);
    Ok(json!({
        "canManage": true,
        "items": items,
    }))
}

#[tauri::command]
pub async fn sheets_delete(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<serde_json::Value, String> {
    require_all_tables_oauth(&state)?;
    let spreadsheet_id = normalize_spreadsheet_id(&id);
    if spreadsheet_id.is_empty() {
        return Err(t("errors.allTables.emptyId"));
    }
    let client = GoogleSheetsClient::from_user_oauth().await?;
    client.delete_spreadsheet(&spreadsheet_id).await?;
    state
        .log
        .info(format!("[AllTables] Удалена таблица {spreadsheet_id}"));
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub async fn sheets_get_permissions(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<serde_json::Value, String> {
    require_all_tables_oauth(&state)?;
    let spreadsheet_id = normalize_spreadsheet_id(&id);
    if spreadsheet_id.is_empty() {
        return Err(t("errors.allTables.emptyId"));
    }
    let client = GoogleSheetsClient::from_user_oauth().await?;
    let items = client
        .list_file_permissions_detail(&spreadsheet_id)
        .await?;
    Ok(json!(items))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetsRemovePermissionArgs {
    pub id: String,
    pub permission_id: String,
}

#[tauri::command]
pub async fn sheets_remove_permission(
    state: State<'_, Arc<AppState>>,
    args: SheetsRemovePermissionArgs,
) -> Result<serde_json::Value, String> {
    require_all_tables_oauth(&state)?;
    let spreadsheet_id = normalize_spreadsheet_id(&args.id);
    let permission_id = args.permission_id.trim().to_string();
    if spreadsheet_id.is_empty() || permission_id.is_empty() {
        return Err(t("errors.allTables.emptyId"));
    }
    let client = GoogleSheetsClient::from_user_oauth().await?;
    client
        .remove_permission(&spreadsheet_id, &permission_id)
        .await?;
    Ok(json!({ "ok": true }))
}
