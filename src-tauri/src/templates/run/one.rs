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

use super::params::build_run_params;
use crate::app_state::AppState;
use crate::core::i18n::{t, tf};
use crate::core::script::{format_label, ScriptContext};
use crate::templates::{runs, store};
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub async fn run_one_template(
    state: &AppState,
    tpl: &crate::templates::Template,
    dry_run: bool,
) -> Result<(usize, String), String> {
    let run_key = format!("template:{}", tpl.id);
    let cancelled = runs::register(&run_key);
    let started_at = Utc::now().to_rfc3339();
    runs::emit_progress("template", &tpl.id, Some("running"), true, Some(&started_at));

    let result = run_one_inner(state, tpl, cancelled.clone(), dry_run).await;

    runs::unregister(&run_key);
    match &result {
        Ok(_) => {
            let at = Utc::now().to_rfc3339();
            runs::emit_progress("template", &tpl.id, Some("ok"), false, Some(&at));
        }
        Err(e) => {
            let status = if cancelled.load(Ordering::Relaxed) {
                "cancelled"
            } else {
                // keep full message for store; UI sees truncated via event
                "error"
            };
            let at = Utc::now().to_rfc3339();
            let status_full = if status == "error" {
                format!("error: {e}")
            } else {
                status.to_string()
            };
            runs::emit_progress(
                "template",
                &tpl.id,
                Some(&status_full),
                false,
                Some(&at),
            );
        }
    }
    result
}

pub(super) async fn run_one_inner(
    state: &AppState,
    tpl: &crate::templates::Template,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    dry_run: bool,
) -> Result<(usize, String), String> {
    crate::templates::access_gate::require_script(state, &tpl.script_key)?;
    let market = crate::templates::model::normalize_marketplace(&tpl.marketplace);
    if !crate::scripts::groups::script_belongs_to_marketplace(&tpl.script_key, &market) {
        return Err(format!(
            "Скрипт «{}» не относится к площадке «{}»",
            tpl.script_key, market
        ));
    }

    let script = state
        .registry
        .get(&tpl.script_key)
        .ok_or_else(|| {
            tf(
                "errors.templates.scriptNotFound",
                &[("scriptKey", tpl.script_key.as_str())],
            )
        })?;

    let config = state.config.lock().map_err(|_| "config lock")?.clone();
    // Пустые поля шаблона → app.config (как resolve_spreadsheet_target); запомним для backfill.
    let resolved_sid = if tpl.spreadsheet_id.trim().is_empty() {
        config.result_spreadsheet_id.clone()
    } else {
        tpl.spreadsheet_id.clone()
    };
    let resolved_sheet = if tpl.sheet_name.trim().is_empty() {
        config.result_sheet_name.clone()
    } else {
        tpl.sheet_name.clone()
    };
    // Без Google — только Excel (UI лочит тумблер). С Google — флаг шаблона.
    let export_to_excel = if crate::templates::access_gate::templates_excel_only(state) {
        true
    } else {
        tpl.export_to_excel
    };
    crate::templates::access_gate::reject_google_write_if_locked(state, export_to_excel)?;
    let need_backfill = !dry_run
        && !export_to_excel
        && (tpl.spreadsheet_id.trim().is_empty()
            || tpl.sheet_name.trim().is_empty()
            || tpl.spreadsheet_title.trim().is_empty());

    let excel_file_path = if export_to_excel && !dry_run {
        Some(make_excel_export_path(&tpl.name)?)
    } else {
        None
    };

    let params = build_run_params(
        tpl,
        &resolved_sid,
        &resolved_sheet,
        export_to_excel,
        excel_file_path.as_deref(),
        dry_run,
    );

    let calc_prefix = t("templates.calculatorPrefix");
    let calc_tab = t("templates.calculatorTab");
    let from_calculator =
        tpl.name.starts_with(&calc_prefix) || tpl.tab_name.eq_ignore_ascii_case(&calc_tab);
    let script_name = if from_calculator {
        let stripped = tpl.name.trim_start_matches(calc_prefix.as_str()).trim();
        if stripped.is_empty() {
            script.title()
        } else {
            stripped.to_string()
        }
    } else {
        script.title()
    };
    let log_label = format_label(&script_name, from_calculator);
    let ctx = ScriptContext::build_with_cancel(
        state.log.clone(),
        state.http.clone(),
        state.rate_limiter.clone(),
        config,
        params,
        tpl.script_key.clone(),
        log_label,
        cancelled,
    );
    // Sheets write needs user OAuth — fail before cabinet fetch spam.
    if !dry_run && !export_to_excel {
        if !crate::core::google::oauth::has_cached_token() {
            return Err(crate::core::i18n::t(
                "errors.google.notConnectedHint",
            ));
        }
    }

    state
        .log
        .info(tf(
            "log.templates.runStart",
            &[
                ("name", tpl.name.as_str()),
                ("scriptKey", tpl.script_key.as_str()),
            ],
        ));
    match script.execute(&ctx).await {
        Ok(out) => {
            let status = if dry_run { "ok:dry_run" } else { "ok" };
            let _ = store::set_last_run(&tpl.id, status);
            // Prefixed like script lines: `[WB: Заказы]: Шаблон «…» успешно выполнен`.
            ctx.log.info(tf(
                "log.templates.runSuccess",
                &[("name", tpl.name.as_str())],
            ));
            // Lesson 123: never block finish emit on opener / Sheets meta (spawn both).
            if let Some(path) = excel_file_path.clone() {
                let log = state.log.clone();
                tauri::async_runtime::spawn(async move {
                    if !std::path::Path::new(&path).is_file() {
                        return;
                    }
                    match tauri_plugin_opener::open_path(&path, None::<&str>) {
                        Ok(()) => log.info(tf(
                            "log.templates.excelOpened",
                            &[("path", path.as_str())],
                        )),
                        Err(e) => log.warn(tf(
                            "log.templates.excelOpenFailed",
                            &[("error", &e.to_string())],
                        )),
                    }
                });
            }
            if need_backfill {
                let tpl_id = tpl.id.clone();
                let sid = resolved_sid.clone();
                let sheet = resolved_sheet.clone();
                let mut title = tpl.spreadsheet_title.clone();
                tauri::async_runtime::spawn(async move {
                    if title.trim().is_empty() && !sid.trim().is_empty() {
                        if let Ok(client) =
                            crate::core::google::sheets::GoogleSheetsClient::from_user_oauth().await
                        {
                            if let Ok((doc_title, _)) = client.list_spreadsheet_meta(&sid).await {
                                title = doc_title;
                            }
                        }
                    }
                    let _ = store::backfill_spreadsheet_target(&tpl_id, &sid, &sheet, &title);
                });
            }
            Ok((out.rows_written, out.message))
        }
        Err(e) => {
            let msg = e.to_string();
            let cancelled_msg = t("errors.cancelled");
            let status = if msg.contains(&cancelled_msg) {
                "cancelled".to_string()
            } else {
                format!("error: {msg}")
            };
            let _ = store::set_last_run(&tpl.id, &status);
            Err(msg)
        }
    }
}

pub(super) async fn run_one_with_flag(
    state: &AppState,
    tpl: &crate::templates::Template,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    dry_run: bool,
    group_id: Option<&str>,
) -> Result<(usize, String), String> {
    let started_at = Utc::now().to_rfc3339();
    // Nested group runs must carry groupId so UI wave can advance without snap lookup.
    runs::emit_progress_ex(
        "template",
        &tpl.id,
        Some("running"),
        true,
        Some(&started_at),
        None,
        group_id,
    );
    let result = run_one_inner(state, tpl, cancelled.clone(), dry_run).await;
    let at = Utc::now().to_rfc3339();
    match &result {
        Ok(_) => runs::emit_progress_ex(
            "template",
            &tpl.id,
            Some("ok"),
            false,
            Some(&at),
            None,
            group_id,
        ),
        Err(e) => {
            let cancelled_msg = t("errors.cancelled");
            let status = if cancelled.load(Ordering::Relaxed) || e.contains(&cancelled_msg) {
                "cancelled".to_string()
            } else {
                format!("error: {e}")
            };
            runs::emit_progress_ex(
                "template",
                &tpl.id,
                Some(&status),
                false,
                Some(&at),
                None,
                group_id,
            );
        }
    }
    result
}

/// `%TEMP%/StatHive/exports/{safeName}_{yyyyMMdd_HHmmss}.xlsx` — паритет ExportToExcel.
fn make_excel_export_path(template_name: &str) -> Result<String, String> {
    let temp = std::env::temp_dir();
    let dir = temp.join("StatHive").join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = Utc::now().format("%Y%m%d_%H%M%S");
    let mut safe: String = template_name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    safe = safe.trim().trim_matches('.').to_string();
    if safe.is_empty() {
        safe = "template".into();
    }
    if safe.len() > 80 {
        safe.truncate(80);
    }
    Ok(dir
        .join(format!("{safe}_{stamp}.xlsx"))
        .to_string_lossy()
        .into_owned())
}
