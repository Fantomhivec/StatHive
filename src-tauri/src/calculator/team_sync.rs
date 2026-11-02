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

//! Периодическая подстановка назначенной конфигурации + событие UI.

use crate::calculator::profile::CalculatorProfile;
use crate::calculator::team_config::{self, TeamConfigEntry};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use crate::core::log::AppLog;
use serde_json::json;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: AppHandle) {
    let _ = APP.set(app);
}

fn emit_applied(entry: &TeamConfigEntry) {
    if let Some(app) = APP.get() {
        let _ = app.emit(
            "wbCalculator.teamConfigApplied",
            json!({
                "configId": entry.id,
                "name": entry.name,
                "revision": entry.revision,
            }),
        );
    }
}

/// Прочитать назначение пользователя и при необходимости записать локальный профиль.
/// Возвращает `true`, если профиль обновили.
///
/// Admin: если локально ещё нет stamp (новая команда / пустой слот) и в CalcTeam есть
/// конфигурации — подтянуть default. Иначе Admin правит локально сам (таймер не затирает).
/// Участник: назначенная в Users конфигурация (или default).
pub async fn sync_assigned_config(
    sa: &GoogleSheetsClient,
    workspace_id: &str,
    row_index: u32,
    access_level: &str,
    log: &AppLog,
) -> Result<bool, String> {
    let catalog = team_config::load_catalog(sa).await?;

    if crate::core::access::is_admin_role(access_level) {
        let stamp = team_config::load_stamp();
        if !stamp.config_id.trim().is_empty() {
            return Ok(false);
        }
        let Some(mut entry) = team_config::resolve_entry(&catalog, "").cloned() else {
            return Ok(false);
        };
        // Новая команда: кабинеты/ставки из default, spreadsheetId пустые (свои таблицы).
        strip_sheet_bindings_for_new_team(&mut entry.profile);
        team_config::apply_entry_locally(&entry)?;
        emit_applied(&entry);
        log.info(tf(
            "log.calc.teamPulledNamed",
            &[("name", &entry.name), ("id", &entry.id)],
        ));
        return Ok(true);
    }

    if row_index < 2 {
        return Ok(false);
    }
    let assigned = crate::auth::sheets_users::read_user_calc_config(sa, workspace_id, row_index)
        .await
        .unwrap_or_default();
    let entry = if assigned.trim().is_empty() {
        team_config::resolve_entry(&catalog, "").cloned()
    } else {
        team_config::find_by_id(&catalog, &assigned).cloned()
    };
    let Some(mut entry) = entry else {
        log.info(t("log.calc.teamPullSkip"));
        return Ok(false);
    };
    // Без флага tables — пустые tableLinks (как Drive reader).
    entry.profile = team_config::profile_for_user_drive(access_level, &entry.profile);
    let local = crate::calculator::profile::load();
    let links_changed = local.table_links != entry.profile.table_links;
    if !team_config::needs_apply(&entry) && !links_changed {
        return Ok(false);
    }
    team_config::apply_entry_locally(&entry)?;
    emit_applied(&entry);
    log.info(tf(
        "log.calc.teamPulledNamed",
        &[("name", &entry.name), ("id", &entry.id)],
    ));
    Ok(true)
}

/// Первый pull в пустой слот: оставить кабинеты/формулы, сбросить id таблиц.
fn strip_sheet_bindings_for_new_team(profile: &mut CalculatorProfile) {
    profile.table_links.clear();
    profile.last_updated.clear();
    profile.template_ids_by_table_key.clear();
    for b in &mut profile.column_bindings {
        b.spreadsheet_url.clear();
    }
}
