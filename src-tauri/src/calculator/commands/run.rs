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
use crate::calculator::{access_guard, excel_export, export_service, profile, run_log, snapshot_mut};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportExcelArgs {
    pub path: String,
}

#[tauri::command]
pub async fn wb_calculator_export_excel(
    state: State<'_, Arc<AppState>>,
    args: ExportExcelArgs,
) -> Result<Value, String> {
    // excel alone не включает calc shell — нужен любой calc-флаг вкладки + excel.
    access_guard::require_any_calc_tab(&state)?;
    access_guard::require_excel(&state)?;
    let _mirror = run_log::enable_app_log_mirror();
    let profile = profile::load();
    let gs = super::calc_sheets(&state).await?;
    excel_export::export_excel_logged(&profile, &gs, &args.path, Some(&state.log)).await
}

#[derive(Debug, Deserialize)]
pub struct OpenTableArgs {
    pub key: String,
}

#[tauri::command]
pub fn wb_calculator_open_table(
    state: State<'_, Arc<AppState>>,
    args: OpenTableArgs,
) -> Result<Value, String> {
    access_guard::require_tables(&state)?;
    let url = snapshot_mut::open_table_url(&args.key)?;
    Ok(serde_json::json!({ "ok": true, "url": url }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrUpdateArgs {
    #[serde(default)]
    pub marketplace: Option<String>,
}

#[tauri::command]
pub async fn wb_calculator_create_or_update_all(
    state: State<'_, Arc<AppState>>,
    args: CreateOrUpdateArgs,
) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    export_service::create_or_update_all(state.inner().clone(), args.marketplace.as_deref()).await
}

#[derive(Debug, Deserialize)]
pub struct RefreshTableArgs {
    pub key: String,
}

#[tauri::command]
pub async fn wb_calculator_refresh_table(
    state: State<'_, Arc<AppState>>,
    args: RefreshTableArgs,
) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    export_service::refresh_table(state.inner().clone(), &args.key).await
}

#[tauri::command]
pub fn wb_calculator_get_run_logs(state: State<'_, Arc<AppState>>) -> Result<Value, String> {
    access_guard::require_any_calc_tab(&state)?;
    Ok(serde_json::json!({
        "lines": run_log::lines(),
        "text": run_log::text()
    }))
}

#[tauri::command]
pub fn wb_calculator_clear_run_logs(state: State<'_, Arc<AppState>>) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    run_log::clear();
    Ok(serde_json::json!({ "ok": true }))
}
