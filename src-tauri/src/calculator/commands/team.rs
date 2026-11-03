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
use crate::calculator::team_config;
use crate::core::access;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::t;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn wb_calculator_list_team_configs(
    state: State<'_, Arc<AppState>>,
) -> Result<Value, String> {
    let u = state
        .auth
        .current_user()
        .filter(|u| u.is_authorized)
        .ok_or_else(|| t("errors.auth.notAuthorized"))?;
    if !access::can_users_tab(&u.access_level) && !access::can_manage_team(&u.access_level) {
        return Err(t("errors.auth.teamListAdminOnly"));
    }
    let client = crate::auth::sheets_users::auth_client().await?;
    let catalog = team_config::load_catalog(&client).await?;
    Ok(serde_json::json!({
        "defaultId": catalog.default_id,
        "items": team_config::list_items(&catalog),
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTeamConfigArgs {
    pub name: String,
    #[serde(default)]
    pub make_default: bool,
}

/// Admin: локальный профиль → именованная запись CalcTeam (+ Drive для уже назначенных).
#[tauri::command]
pub async fn wb_calculator_publish_team_config(
    state: State<'_, Arc<AppState>>,
    args: PublishTeamConfigArgs,
) -> Result<Value, String> {
    super::require_admin(&state)?;
    let client = crate::auth::sheets_users::auth_client().await?;
    let old_catalog = team_config::load_catalog(&client).await?;
    let old_entry = team_config::find_by_name(&old_catalog, &args.name).cloned();
    let entry = team_config::publish(&client, &args.name, args.make_default).await?;
    // Переиздание: SA читает таблицы за участников; затем reader на их email.
    if let Ok(sid) = crate::auth::sheets_users::spreadsheet_id() {
        if let Ok(users) = crate::auth::sheets_users::list_users(&client, &sid).await {
            if let Ok(drive) = GoogleSheetsClient::from_user_oauth().await {
                let title = format!("Конфигурация «{}»", entry.name);
                if let Err(e) = team_config::share_tables_with_service_account(
                    &drive,
                    &entry.profile,
                    &state.log,
                    &title,
                    &team_config::keep_going,
                )
                .await
                {
                    state.log.warn(format!("{title}: {e}"));
                }
                for u in users {
                    if !u.calc_config_id.eq_ignore_ascii_case(&entry.id) {
                        continue;
                    }
                    let email = u.google_email.trim();
                    if email.is_empty() {
                        continue;
                    }
                    // Drive reader только при CalcAccess.tables (как invite/assign).
                    let new_drive = team_config::profile_for_user_drive(
                        &u.access_level,
                        &entry.profile,
                    );
                    let old_drive = old_entry.as_ref().map(|e| {
                        team_config::profile_for_user_drive(&u.access_level, &e.profile)
                    });
                    let who = format!("Конфигурация «{}», {}", entry.name, u.login);
                    if let Err(e) = team_config::sync_drive_access(
                        &drive,
                        email,
                        old_drive.as_ref(),
                        &new_drive,
                        &state.log,
                        &who,
                        &team_config::keep_going,
                    )
                    .await
                    {
                        state.log.warn(format!("{who}: {e}"));
                    }
                }
            }
        }
    }
    state.log.info(t("log.calc.teamPublished"));
    Ok(serde_json::json!({
        "ok": true,
        "id": entry.id,
        "name": entry.name,
        "revision": entry.revision,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfigIdArgs {
    pub id: String,
}

#[tauri::command]
pub async fn wb_calculator_set_default_team_config(
    state: State<'_, Arc<AppState>>,
    args: TeamConfigIdArgs,
) -> Result<Value, String> {
    super::require_admin(&state)?;
    let client = crate::auth::sheets_users::auth_client().await?;
    team_config::set_default(&client, &args.id).await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub async fn wb_calculator_delete_team_config(
    state: State<'_, Arc<AppState>>,
    args: TeamConfigIdArgs,
) -> Result<Value, String> {
    super::require_admin(&state)?;
    let client = crate::auth::sheets_users::auth_client().await?;
    team_config::delete_config(&client, &args.id).await?;
    Ok(serde_json::json!({ "ok": true }))
}

/// Загрузить конфигурацию в локальный профиль (Admin / редакторы — для проверки).
#[tauri::command]
pub async fn wb_calculator_pull_team_config(
    state: State<'_, Arc<AppState>>,
    args: TeamConfigIdArgs,
) -> Result<Value, String> {
    super::require_admin(&state)?;
    let client = crate::auth::sheets_users::auth_client().await?;
    let Some(entry) = team_config::pull_by_id(&client, &args.id).await? else {
        return Ok(serde_json::json!({ "ok": true, "pulled": false }));
    };
    Ok(serde_json::json!({
        "ok": true,
        "pulled": true,
        "id": entry.id,
        "name": entry.name,
        "revision": entry.revision,
    }))
}
