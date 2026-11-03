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
use crate::calculator::{access_guard, snapshot};
use crate::core::access;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

fn load_snapshot_for_user(state: &AppState, view: &str) -> Result<Value, String> {
    access_guard::require_any_calc_tab(state)?;
    let user = state.auth.current_user();
    let admin = user
        .as_ref()
        .map(|u| access::is_admin_role(&u.access_level))
        .unwrap_or(false);
    let mut snap = snapshot::get_snapshot_for_view(admin, view);
    // Срез полей по CalcAccess (как вкладки UI): не отдавать чужие блоки.
    if let Some(u) = user.as_ref() {
        if !access::is_admin_role(&u.access_level) {
            if let Some(obj) = snap.as_object_mut() {
                let level = &u.access_level;
                if !access::can_calc(level, access::CALC_FLAG_TABLES) {
                    obj.insert("tableLinks".into(), serde_json::json!({}));
                    obj.insert("tableModes".into(), serde_json::json!({}));
                    obj.insert("columnBindings".into(), serde_json::json!([]));
                    obj.insert("lastUpdated".into(), serde_json::json!({}));
                    obj.insert("templateIdsByTableKey".into(), serde_json::json!({}));
                }
                if !access::can_calc(level, access::CALC_FLAG_SETTINGS) {
                    for k in [
                        "taxPercent",
                        "acquiringPercent",
                        "advPercent",
                        "extraExpensesPercent",
                        "ozonTaxPercent",
                        "ozonAcquiringPercent",
                        "ozonAdvPercent",
                        "ozonExtraExpensesPercent",
                        "profitFormula",
                        "ozonProfitFormula",
                        "triggersEnabled",
                    ] {
                        obj.remove(k);
                    }
                }
                if !access::can_calc(level, access::CALC_FLAG_KGT) {
                    obj.remove("wbKgt");
                    obj.remove("ozKgt");
                }
            }
        }
    }
    Ok(snap)
}

#[tauri::command]
pub fn wb_calculator_get_snapshot(state: State<'_, Arc<AppState>>) -> Result<Value, String> {
    load_snapshot_for_user(&state, "full")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotForArgs {
    #[serde(default)]
    pub view: String,
}

#[tauri::command]
pub fn wb_calculator_get_snapshot_for(
    state: State<'_, Arc<AppState>>,
    args: SnapshotForArgs,
) -> Result<Value, String> {
    // Собираем только ключи view — без полного JSON + retain.
    load_snapshot_for_user(&state, &args.view)
}
