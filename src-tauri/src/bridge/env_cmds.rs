use crate::app_state::AppState;
use crate::core::access;
use crate::core::i18n::{t, tf};
use crate::core::env_check::{self, KeyCheckResult};
use crate::core::env_keys::{self, EnvKeyRow, EnvKeysSnapshot};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::spreadsheet_id;
use crate::core::wb_token;
use chrono::Local;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

fn env_path_from_state(state: &AppState) -> Result<std::path::PathBuf, String> {
    let mut cfg = state.config.lock().map_err(|_| "config lock")?;
    // Только *.env рядом с exe (keys.env, cab.env, .env, …).
    let raw = cfg.env_file_path.trim().to_string();
    if !raw.is_empty() {
        if is_appdata_env_path(&raw) {
            cfg.env_file_path.clear();
            let _ = cfg.save();
        } else if let Ok(p) = crate::core::paths::ensure_env_beside_exe(std::path::Path::new(&raw))
        {
            return Ok(p);
        } else {
            cfg.env_file_path.clear();
            let _ = cfg.save();
        }
    }
    Ok(crate::core::paths::env_file_path())
}

fn is_appdata_env_path(raw: &str) -> bool {
    let p = raw.trim();
    if p.is_empty() {
        return false;
    }
    let lower = p.replace('/', "\\").to_ascii_lowercase();
    lower.contains("\\appdata\\")
        && (lower.contains("\\stathive\\") || lower.contains("\\universalrust\\"))
        && lower.ends_with("\\.env")
}

fn require_keys_check(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if !access::can_keys_check(&u.access_level) {
        return Err(t("errors.auth.keysCheckDenied"));
    }
    Ok(())
}

/// KeysCheck **или** (OAuth ∧ скрипты) — Google list/check (нужен token).
fn require_keys_or_script_oauth(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if access::can_keys_check(&u.access_level) {
        return Ok(());
    }
    let oauth = access::effective_has_oauth(&u.access_level, u.has_oauth);
    let any_member_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &u.access_level);
    if oauth && access::can_use_any_script_ex(&u.access_level, oauth, any_member_ok) {
        return Ok(());
    }
    Err(t("errors.auth.keysCheckDenied"))
}

/// KeysCheck **или** любой разрешённый скрипт (в т.ч. Excel-only / memberOk) — имена кабинетов без секретов.
fn require_keys_or_any_script(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if access::can_keys_check(&u.access_level) {
        return Ok(());
    }
    let oauth = access::effective_has_oauth(&u.access_level, u.has_oauth);
    let any_member_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &u.access_level);
    if access::can_use_any_script_ex(&u.access_level, oauth, any_member_ok) {
        return Ok(());
    }
    Err(t("errors.auth.keysCheckDenied"))
}

fn emit_tabs_layout(app: &tauri::AppHandle, state: &AppState) {
    use tauri::Emitter;
    let Some(user) = state.auth.current_user().filter(|u| u.is_authorized) else {
        return;
    };
    let any_ok =
        crate::templates::access_gate::any_member_ok_in_scope(state, &user.access_level);
    let oauth = access::effective_has_oauth(&user.access_level, user.has_oauth);
    let can_check =
        crate::core::oauth_config::can_check_google(&user.access_level, &user.login);
    let layout = crate::core::tabs_layout::get_layout(
        &user.login,
        &user.access_level,
        oauth,
        any_ok,
        can_check,
    );
    let _ = app.emit("tabsLayout.changed", &layout);
}

/// Имена кабинетов без plaintext секретов — для редактора шаблонов (scripts+OAuth без keysCheck).
#[tauri::command]
pub fn env_list_cabinet_names(
    state: State<'_, Arc<AppState>>,
) -> Result<EnvKeysSnapshot, String> {
    require_keys_or_any_script(&state)?;
    let path = env_path_from_state(&state)?;
    let mut snap = env_keys::list_rows(&path)?;
    for row in &mut snap.rows {
        row.value.clear();
    }
    Ok(snap)
}

#[tauri::command]
pub fn env_check_keys(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<KeyCheckResult, String> {
    require_keys_check(&state)?;
    let path = env_path_from_state(&state)?;
    let res = env_check::check_keys(&path)?;
    emit_tabs_layout(&app, &state);
    Ok(res)
}

/// Ensure `.env` exists and return rows (one round-trip for editor open).
#[tauri::command]
pub fn env_create_default(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<EnvKeysSnapshot, String> {
    require_keys_check(&state)?;
    let path = env_path_from_state(&state)?;
    let snap = env_keys::create_empty(&path)?;
    emit_tabs_layout(&app, &state);
    Ok(snap)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvSetPathArgs {
    pub path: String,
}

/// Запомнить `.env` рядом с exe (после выбора папки приложения).
#[tauri::command]
pub fn env_set_path(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    args: EnvSetPathArgs,
) -> Result<serde_json::Value, String> {
    require_keys_check(&state)?;
    let p = args.path.trim();
    if p.is_empty() {
        return Err(t("errors.env.pathRequired"));
    }
    let path = crate::core::paths::ensure_env_beside_exe(std::path::Path::new(p))?;
    if !path.is_file() {
        return Err(t("errors.env.fileNotFound"));
    }
    // Убеждаемся, что файл читается (старый sealed-формат → plaintext или ошибка).
    let _ = env_keys::list_rows(&path)?;
    crate::core::paths::purge_stray_env_copies();
    let display = path.display().to_string();
    if let Ok(mut cfg) = state.config.lock() {
        cfg.env_file_path = display.clone();
        cfg.save()?;
    }
    emit_tabs_layout(&app, &state);
    Ok(serde_json::json!({ "path": display }))
}

/// Каталог exe — стартовая папка диалога «Выбрать .env».
#[tauri::command]
pub fn env_exe_dir() -> String {
    crate::core::paths::exe_dir().display().to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvPathArgs {
    pub path: String,
}

/// Скопировать текущий `.env` в выбранный путь (экспорт бэкапа).
#[tauri::command]
pub fn env_export_to(
    state: State<'_, Arc<AppState>>,
    args: EnvPathArgs,
) -> Result<serde_json::Value, String> {
    require_keys_check(&state)?;
    let dest = args.path.trim();
    if dest.is_empty() {
        return Err(t("errors.env.pathRequired"));
    }
    let src = env_path_from_state(&state)?;
    if !src.is_file() {
        return Err(t("errors.env.fileNotFound"));
    }
    let dest_path = std::path::PathBuf::from(dest);
    // Экспорт всегда plaintext — удобный бэкап на другой носитель.
    env_keys::export_plaintext(&src, &dest_path)?;
    Ok(serde_json::json!({
        "ok": true,
        "path": dest_path.display().to_string(),
    }))
}

/// Импорт ключей из другого `.env` в текущий файл (merge по env_key / кабинет).
#[tauri::command]
pub fn env_import_from(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    args: EnvPathArgs,
) -> Result<EnvKeysSnapshot, String> {
    require_keys_check(&state)?;
    let src = args.path.trim();
    if src.is_empty() {
        return Err(t("errors.env.pathRequired"));
    }
    let src_path = std::path::PathBuf::from(src);
    if !src_path.is_file() {
        return Err(t("errors.env.fileNotFound"));
    }
    let current = env_path_from_state(&state)?;
    let incoming = env_keys::list_rows(&src_path)?;
    let mut snap = env_keys::list_rows(&current)?;
    for row in incoming.rows {
        if let Some(pos) = snap
            .rows
            .iter()
            .position(|r| r.env_key.eq_ignore_ascii_case(&row.env_key))
        {
            snap.rows[pos] = row;
        } else {
            snap.rows.push(row);
        }
    }
    let saved = env_keys::save_rows(&current, &snap.rows)?;
    emit_tabs_layout(&app, &state);
    Ok(saved)
}

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
            // auto=true при заходе на вкладку — не засоряем лог; ручная проверка — warn.
            if !args.auto {
                state.log.warn(tf(
                    "log.google.notConnected",
                    &[("error", &oauth_err.to_string())],
                ));
            }
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

#[derive(Deserialize)]
pub struct SaveEnvRowsArgs {
    pub rows: Vec<EnvKeyRow>,
}

#[tauri::command]
pub fn env_save_rows(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    args: SaveEnvRowsArgs,
) -> Result<EnvKeysSnapshot, String> {
    require_keys_check(&state)?;
    let path = env_path_from_state(&state)?;
    let snap = env_keys::save_rows(&path, &args.rows)?;
    let wb: Vec<_> = snap
        .rows
        .iter()
        .filter(|r| r.marketplace.eq_ignore_ascii_case("WB"))
        .map(|r| r.cabinet.as_str())
        .collect();
    if !wb.is_empty() {
        state
            .log
            .info(tf(
                "log.env.cabinetsSelected",
                &[("cabinets", &wb.join(", "))],
            ));
    }
    emit_tabs_layout(&app, &state);
    Ok(snap)
}

#[derive(Deserialize)]
pub struct WbTokenInfoArgs {
    pub cabinet: String,
    pub marketplace: String,
    pub value: String,
}

#[tauri::command]
pub fn env_wb_token_info(
    state: State<'_, Arc<AppState>>,
    args: WbTokenInfoArgs,
) -> Result<serde_json::Value, String> {
    require_keys_check(&state)?;
    if !args.marketplace.eq_ignore_ascii_case("WB") {
        return Ok(serde_json::json!({
            "ok": false,
            "title": t("errors.google.tokenInfoTitle"),
            "message": t("errors.google.tokenInfoWbOnly")
        }));
    }
    match wb_token::decode(args.value.trim()) {
        Ok(info) => Ok(serde_json::json!({
            "ok": true,
            "title": t("errors.google.wbTokenInfoTitle"),
            "message": wb_token::info_text(&args.cabinet, &info),
        })),
        Err(e) => Ok(serde_json::json!({
            "ok": false,
            "title": t("errors.google.tokenInfoTitle"),
            "message": tf("errors.google.errorMessage", &[("error", &e.to_string())]),
        })),
    }
}

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

