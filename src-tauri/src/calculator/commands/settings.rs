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
use crate::calculator::{access_guard, config_io, snapshot_mut};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn wb_calculator_save_settings(
    state: State<'_, Arc<AppState>>,
    settings: Value,
) -> Result<Value, String> {
    access_guard::require_save_settings(&state, &settings)?;
    snapshot_mut::save_settings(&settings)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTriggersArgs {
    #[serde(default)]
    pub triggers: Value,
}

#[tauri::command]
pub fn wb_calculator_set_triggers(
    state: State<'_, Arc<AppState>>,
    args: SetTriggersArgs,
) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    crate::calculator::triggers::set_triggers_from_json(&args.triggers)
}

#[derive(Debug, Deserialize)]
pub struct EditLinkArgs {
    pub key: String,
    #[serde(default)]
    pub url: String,
}

#[tauri::command]
pub fn wb_calculator_edit_table_link(
    state: State<'_, Arc<AppState>>,
    args: EditLinkArgs,
) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    snapshot_mut::edit_table_link(&args.key, &args.url)
}

#[tauri::command]
pub async fn wb_calculator_clear_table_links(
    state: State<'_, Arc<AppState>>,
) -> Result<Value, String> {
    access_guard::require_tables_mutate(&state)?;
    snapshot_mut::clear_table_links().await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitTokenArgs {
    pub token: String,
    #[serde(default)]
    pub formula_field: Option<String>,
}

#[tauri::command]
pub fn wb_calculator_append_profit_token(
    state: State<'_, Arc<AppState>>,
    args: ProfitTokenArgs,
) -> Result<Value, String> {
    access_guard::require_settings(&state)?;
    snapshot_mut::append_profit_token(&args.token, args.formula_field.as_deref())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearFormulaArgs {
    #[serde(default)]
    pub formula_field: Option<String>,
}

#[tauri::command]
pub fn wb_calculator_clear_profit_formula(
    state: State<'_, Arc<AppState>>,
    args: ClearFormulaArgs,
) -> Result<Value, String> {
    access_guard::require_settings(&state)?;
    snapshot_mut::clear_profit_formula(args.formula_field.as_deref())
}

#[derive(Debug, Deserialize)]
pub struct ConfigPathArgs {
    #[serde(default)]
    pub path: String,
}

#[tauri::command]
pub fn wb_calculator_export_config_file(
    state: State<'_, Arc<AppState>>,
    args: ConfigPathArgs,
) -> Result<Value, String> {
    access_guard::require_settings(&state)?;
    let user = state.auth.current_user();
    let level = user.as_ref().map(|u| u.access_level.as_str());
    config_io::export_config_file_for_access(&args.path, level)
}

#[derive(Debug, Deserialize)]
pub struct ImportConfigArgs {
    #[serde(default)]
    pub json: String,
}

#[tauri::command]
pub fn wb_calculator_import_config(
    state: State<'_, Arc<AppState>>,
    args: ImportConfigArgs,
) -> Result<Value, String> {
    // Полный профиль всегда содержит tableLinks / modes — как save_settings с table-ключами.
    access_guard::require_settings(&state)?;
    access_guard::require_tables_mutate(&state)?;
    config_io::import_config(&args.json)
}

#[tauri::command]
pub fn wb_calculator_import_config_file(
    state: State<'_, Arc<AppState>>,
    args: ConfigPathArgs,
) -> Result<Value, String> {
    access_guard::require_settings(&state)?;
    access_guard::require_tables_mutate(&state)?;
    config_io::import_config_file(&args.path)
}
