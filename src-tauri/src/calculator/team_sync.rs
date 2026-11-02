//! Периодическая подстановка назначенной конфигурации + событие UI.

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
pub async fn sync_assigned_config(
    sa: &GoogleSheetsClient,
    workspace_id: &str,
    row_index: u32,
    access_level: &str,
    log: &AppLog,
) -> Result<bool, String> {
    // Admin сам переключает сохранённые снимки; таймер его локальный профиль не затирает.
    if crate::core::access::is_admin_role(access_level) {
        return Ok(false);
    }
    if row_index < 2 {
        return Ok(false);
    }
    let assigned = crate::auth::sheets_users::read_user_calc_config(sa, workspace_id, row_index)
        .await
        .unwrap_or_default();
    let catalog = team_config::load_catalog(sa).await?;
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
