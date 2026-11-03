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
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

use super::guards::require_keys_or_script_oauth;

#[derive(Deserialize)]
pub struct GoogleCheckArgs {
    #[serde(default)]
    pub spreadsheet_id: String,
    #[serde(default)]
    pub auto: bool,
}

#[tauri::command]
pub async fn google_check_connection(
    state: State<'_, Arc<AppState>>,
    args: GoogleCheckArgs,
) -> Result<serde_json::Value, String> {
    require_keys_or_script_oauth(&state)?;
    let _ = args.spreadsheet_id;
    let time = Local::now().format("%H:%M:%S").to_string();

    let client = match GoogleSheetsClient::from_user_oauth().await {
        Ok(c) => c,
        Err(oauth_err) => {
            // auto=true при заходе на вкладку — не засоряем лог и не пугаем «Ошибка».
            if args.auto {
                return Ok(serde_json::json!({
                    "success": false,
                    "status": t("errors.google.notConnectedStatus"),
                    "message": t("errors.google.notConnectedHint"),
                    "time": time,
                    "tone": "warn",
                }));
            }
            state.log.warn(tf(
                "log.google.notConnected",
                &[("error", &oauth_err.to_string())],
            ));
            return Ok(serde_json::json!({
                "success": false,
                "status": t("errors.google.statusError"),
                "message": tf("errors.google.errorMessage", &[("error", &oauth_err.to_string())]),
                "time": time,
                "tone": "danger",
            }));
        }
    };

    match client.test_connection().await {
        Ok(()) => {
            if !args.auto {
                state.log.info(t("log.google.checkOk"));
            }
            let (status, message) = if args.auto {
                (
                    t("errors.google.connectedTokenValid"),
                    t("errors.google.authSuccess"),
                )
            } else {
                (
                    t("errors.google.connectedApiConfirmed"),
                    t("errors.google.sheetsApiConfirmed"),
                )
            };
            Ok(serde_json::json!({
                "success": true,
                "status": status,
                "message": message,
                "time": time,
                "tone": "success",
            }))
        }
        Err(e) => {
            state.log.warn(tf(
                "log.google.checkFailed",
                &[("error", &e.to_string())],
            ));
            Ok(serde_json::json!({
                "success": false,
                "status": t("errors.google.statusError"),
                "message": tf("errors.google.errorMessage", &[("error", &e.to_string())]),
                "time": time,
                "tone": "danger",
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct ListSheetsArgs {
    pub spreadsheet_id: String,
}

#[tauri::command]
pub async fn google_list_sheets(
    state: State<'_, Arc<AppState>>,
    args: ListSheetsArgs,
) -> Result<serde_json::Value, String> {
    require_keys_or_script_oauth(&state)?;
    let (sid, _) = crate::core::spreadsheet_id::normalize(&args.spreadsheet_id);
    if sid.is_empty() {
        return Err(t("errors.env.spreadsheetIdRequired"));
    }
    let client = GoogleSheetsClient::from_user_oauth().await?;
    let (title, sheets) = client.list_spreadsheet_meta(&sid).await?;
    Ok(serde_json::json!({
        "spreadsheet_id": sid,
        "title": title,
        "sheets": sheets
    }))
}
