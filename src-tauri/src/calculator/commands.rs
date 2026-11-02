//! Tauri IPC для калькулятора.

use crate::app_state::AppState;
use crate::calculator::{
    access_guard, card_reader, config_io, excel_export, export_service, profile, run_log, snapshot,
    snapshot_mut, team_config,
};
use crate::core::access;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::t;
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
pub struct LookupArgs {
    pub article: String,
    #[serde(default)]
    pub marketplace: Option<String>,
    /// Игнорировать RAM-свод и перечитать с Google Sheets.
    #[serde(default)]
    pub force: bool,
}

#[tauri::command]
pub async fn wb_calculator_lookup_card(
    state: State<'_, Arc<AppState>>,
    args: LookupArgs,
) -> Result<Value, String> {
    access_guard::require_view(&state)?;
    let log = &state.log;
    let profile = profile::load();
    let gs = calc_sheets(&state).await?;

    let tables_mp = snapshot::resolve_tables_marketplace(&profile);
    let requested = args
        .marketplace
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let sheets_ready = crate::calculator::article_index::has_sheets();
    // Индекс/листы: только если both/auto и RAM ещё пуста (прогрев обычно на входе в калькулятор).
    let needs_index = !(tables_mp == "wb" || tables_mp == "ozon")
        && (requested.is_empty() || requested == "auto" || requested == "both")
        && !sheets_ready;

    if args.force {
        crate::calculator::article_index::invalidate_sheets();
    }

    let mut cache = crate::calculator::sheet_util::SheetCache::new();
    crate::calculator::article_index::fill_local_cache(&mut cache);
    if needs_index || args.force {
        crate::calculator::article_index::warm_from_summaries(
            &profile,
            &gs,
            &mut cache,
            Some(log),
        )
        .await;
    }

    let resolved = if tables_mp == "wb" || tables_mp == "ozon" {
        tables_mp
    } else if !requested.is_empty() && requested != "auto" && requested != "both" {
        requested
    } else {
        crate::calculator::article_index::resolve(&args.article)
            .0
            .unwrap_or("wb")
            .to_string()
    };

    Ok(card_reader::load_card(
        &profile,
        &args.article,
        Some(&resolved),
        &gs,
        &mut cache,
    )
    .await)
}

/// Фоновый прогрев сводов в RAM (вход на страницу калькулятора). Идемпотентно.
#[tauri::command]
pub async fn wb_calculator_warm_lookup_cache(
    state: State<'_, Arc<AppState>>,
) -> Result<Value, String> {
    access_guard::require_view(&state)?;
    let log = &state.log;
    let keys = crate::calculator::article_index::count();
    if crate::calculator::article_index::has_sheets() && keys > 0 {
        return Ok(serde_json::json!({ "ok": true, "cached": true, "keys": keys }));
    }
    let t0 = std::time::Instant::now();
    let profile = profile::load();
    let gs = calc_sheets(&state).await?;
    let mut cache = crate::calculator::sheet_util::SheetCache::new();
    crate::calculator::article_index::warm_from_summaries(&profile, &gs, &mut cache, Some(log))
        .await;
    let keys = crate::calculator::article_index::count();
    Ok(serde_json::json!({
        "ok": true,
        "cached": false,
        "keys": keys,
        "ms": t0.elapsed().as_millis(),
    }))
}

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
    let gs = calc_sheets(&state).await?;
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

/// Admin / участник с рабочим OAuth — user token; иначе SA (просмотр без своего Google).
async fn calc_sheets(state: &AppState) -> Result<GoogleSheetsClient, String> {
    let use_user_oauth = state
        .auth
        .current_user()
        .map(|u| access::effective_has_oauth(&u.access_level, u.has_oauth))
        .unwrap_or(false);
    if use_user_oauth {
        GoogleSheetsClient::from_user_oauth().await
    } else {
        GoogleSheetsClient::from_service_account().await
    }
}

fn require_admin(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if !access::can_manage_team(&u.access_level) {
        return Err(t("errors.auth.teamCodeAdminOnly"));
    }
    Ok(())
}

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
    require_admin(&state)?;
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
    require_admin(&state)?;
    let client = crate::auth::sheets_users::auth_client().await?;
    team_config::set_default(&client, &args.id).await?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub async fn wb_calculator_delete_team_config(
    state: State<'_, Arc<AppState>>,
    args: TeamConfigIdArgs,
) -> Result<Value, String> {
    require_admin(&state)?;
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
    require_admin(&state)?;
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
