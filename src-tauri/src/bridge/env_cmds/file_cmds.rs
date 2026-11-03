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
use crate::core::env_check::{self, KeyCheckResult};
use crate::core::env_keys::{self, EnvKeyRow, EnvKeysSnapshot};
use crate::core::i18n::{t, tf};
use crate::core::wb_token;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

use super::guards::{
    emit_tabs_layout, env_path_from_state, require_keys_check, require_keys_or_any_script,
};

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
        state.log.info(tf(
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
