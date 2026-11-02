use crate::app_state::AppState;
use crate::core::i18n::{t, tf};
use crate::core::script::{format_label, ScriptContext};
use crate::templates::{runs, store};
use chrono::Utc;
use std::collections::HashMap;
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

async fn run_one_inner(
    state: &AppState,
    tpl: &crate::templates::Template,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    dry_run: bool,
) -> Result<(usize, String), String> {
    crate::templates::access_gate::require_script(state, &tpl.script_key)?;

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

    let mut params = HashMap::new();
    params.insert("spreadsheet_id".into(), resolved_sid.clone());
    params.insert(
        "sheet_name".into(),
        if export_to_excel && resolved_sheet.trim().is_empty() {
            "Sheet1".into()
        } else {
            resolved_sheet.clone()
        },
    );
    if export_to_excel {
        params.insert("export_to_excel".into(), "true".into());
        if let Some(ref path) = excel_file_path {
            params.insert("excel_file_path".into(), path.clone());
        }
    }
    if !tpl.second_spreadsheet_id.trim().is_empty() {
        let sid = tpl.second_spreadsheet_id.clone();
        params.insert("source_spreadsheet_id".into(), sid.clone());
        params.insert("second_spreadsheet_id".into(), sid);
    }
    if !tpl.second_sheet_name.trim().is_empty() {
        let sname = tpl.second_sheet_name.clone();
        params.insert("source_sheet_name".into(), sname.clone());
        params.insert("second_sheet_name".into(), sname);
    }
    if !tpl.id_mode.trim().is_empty() {
        params.insert("id_mode".into(), tpl.id_mode.clone());
    }
    for (key, val) in [
        ("cabinets_source_range", &tpl.cabinets_source_range),
        ("nm_id_source_range", &tpl.nm_id_source_range),
        ("stocks_skus_source_range", &tpl.stocks_skus_source_range),
        (
            "id_adv_campaign_source_range",
            &tpl.id_adv_campaign_source_range,
        ),
        ("offer_id_source_range", &tpl.offer_id_source_range),
        ("product_id_source_range", &tpl.product_id_source_range),
        ("sku_source_range", &tpl.sku_source_range),
        (
            "content_media_links_source_range",
            &tpl.content_media_links_source_range,
        ),
        ("subject_id_source_range", &tpl.subject_id_source_range),
        ("subject_name_source_range", &tpl.subject_name_source_range),
        ("cards_update_length_range", &tpl.cards_update_length_range),
        ("cards_update_width_range", &tpl.cards_update_width_range),
        ("cards_update_height_range", &tpl.cards_update_height_range),
        (
            "cards_update_vendor_code_range",
            &tpl.cards_update_vendor_code_range,
        ),
        ("cards_update_vat_range", &tpl.cards_update_vat_range),
        ("cards_update_brand_range", &tpl.cards_update_brand_range),
        ("prices_price_range", &tpl.prices_price_range),
        ("prices_discount_range", &tpl.prices_discount_range),
        ("prices_exclude_range", &tpl.prices_exclude_range),
        ("prices_raise_percent", &tpl.prices_raise_percent),
        ("price_set_price_range", &tpl.price_set_price_range),
        ("price_set_discount_range", &tpl.price_set_discount_range),
        (
            "prices_history_task_upload_id_range",
            &tpl.prices_history_task_upload_id_range,
        ),
        (
            "paid_storage_status_task_id_range",
            &tpl.paid_storage_status_task_id_range,
        ),
        (
            "paid_storage_status_output_range",
            &tpl.paid_storage_status_output_range,
        ),
        (
            "paid_storage_download_task_id_range",
            &tpl.paid_storage_download_task_id_range,
        ),
        ("symbols_check_range", &tpl.symbols_check_range),
        (
            "report_realization_v5_period",
            &tpl.report_realization_v5_period,
        ),
    ] {
        if !val.trim().is_empty() {
            params.insert(key.into(), val.clone());
        }
    }
    if tpl.use_custom_subject_ids {
        params.insert("use_custom_subject_ids".into(), "true".into());
    }
    if tpl.cells_count_count_filled {
        params.insert("cells_count_count_filled".into(), "true".into());
    }
    if tpl.promotions_all_promo {
        params.insert("promotions_all_promo".into(), "true".into());
    }
    params.insert(
        "cards_update_dimensions_mode".into(),
        if tpl.cards_update_dimensions_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_vendor_code_mode".into(),
        if tpl.cards_update_vendor_code_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_vat_mode".into(),
        if tpl.cards_update_vat_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_brand_mode".into(),
        if tpl.cards_update_brand_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    if let Ok(json) = serde_json::to_string(&tpl.subjects_filters) {
        params.insert("subjects_filters_json".into(), json);
    }
    // Sync Ozon id ranges without cloning the whole Template (selected_fields / triggers).
    let mode = tpl.id_mode.trim().to_ascii_lowercase();
    let legacy = tpl.id_source_range.trim();
    let mut sku_range = tpl.sku_source_range.clone();
    let mut offer_range = tpl.offer_id_source_range.clone();
    let mut product_range = tpl.product_id_source_range.clone();
    {
        let named = match mode.as_str() {
            "offer_id" => &mut offer_range,
            "product_id" => &mut product_range,
            _ => &mut sku_range,
        };
        if named.trim().is_empty() && !legacy.is_empty() {
            *named = legacy.to_string();
        }
    }
    let active = match mode.as_str() {
        "offer_id" => offer_range.trim(),
        "product_id" => product_range.trim(),
        _ => sku_range.trim(),
    };
    if !active.is_empty() {
        params.insert("id_source_range".into(), active.to_string());
        params.insert("sku_source_range".into(), active.to_string());
        match mode.as_str() {
            "offer_id" => {
                params.insert("offer_id_source_range".into(), active.to_string());
            }
            "product_id" => {
                params.insert("product_id_source_range".into(), active.to_string());
            }
            _ => {}
        }
    }
    if !tpl.date_from.trim().is_empty() {
        params.insert("date_from".into(), tpl.date_from.clone());
    }
    if !tpl.date_to.trim().is_empty() {
        params.insert("date_to".into(), tpl.date_to.clone());
    }
    if !tpl.cabinets.is_empty() {
        params.insert("cabinets".into(), tpl.cabinets.join(","));
        params.insert("cabinet".into(), tpl.cabinets[0].clone());
    }
    if let Ok(json) = serde_json::to_string(&tpl.selected_fields) {
        params.insert("selected_fields_json".into(), json);
    }
    if let Ok(json) = serde_json::to_string(&tpl.sync_settings) {
        params.insert("sync_settings_json".into(), json);
    }
    if !tpl.write_sort_field_name.trim().is_empty() {
        params.insert(
            "write_sort_field_name".into(),
            tpl.write_sort_field_name.trim().to_string(),
        );
        let dir = tpl.write_sort_direction.trim().to_ascii_lowercase();
        params.insert(
            "write_sort_direction".into(),
            if dir == "desc" {
                "desc".into()
            } else {
                "asc".into()
            },
        );
    }
    params.insert(
        "clear_only_written_range".into(),
        if tpl.clear_only_written_range {
            "true".into()
        } else {
            "false".into()
        },
    );
    if dry_run {
        params.insert("dry_run".into(), "true".into());
    }

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
            if let Some(ref path) = excel_file_path {
                if std::path::Path::new(path).is_file() {
                    match tauri_plugin_opener::open_path(path, None::<&str>) {
                        Ok(()) => state.log.info(tf(
                            "log.templates.excelOpened",
                            &[("path", path.as_str())],
                        )),
                        Err(e) => state.log.warn(tf(
                            "log.templates.excelOpenFailed",
                            &[("error", &e.to_string())],
                        )),
                    }
                }
            }
            if need_backfill {
                let mut title = tpl.spreadsheet_title.clone();
                if title.trim().is_empty() && !resolved_sid.trim().is_empty() {
                    if let Ok(client) =
                        crate::core::google::sheets::GoogleSheetsClient::from_user_oauth().await
                    {
                        if let Ok((doc_title, _)) = client.list_spreadsheet_meta(&resolved_sid).await
                        {
                            title = doc_title;
                        }
                    }
                }
                let _ = store::backfill_spreadsheet_target(
                    &tpl.id,
                    &resolved_sid,
                    &resolved_sheet,
                    &title,
                );
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

pub async fn run_group(
    state: &AppState,
    group_id: &str,
    dry_run: bool,
) -> Result<(usize, String), String> {
    let group = store::get_group(group_id)?.ok_or_else(|| t("errors.templates.groupNotFound"))?;
    let run_key = format!("group:{}", group.id);
    let cancelled = runs::register(&run_key);
    let started_at = Utc::now().to_rfc3339();
    runs::emit_progress("group", &group.id, Some("running"), true, Some(&started_at));

    let mut total_rows = 0usize;
    let mut messages = Vec::new();
    let mut err: Option<String> = None;

    for tid in &group.template_ids {
        if cancelled.load(Ordering::Relaxed) {
            err = Some(t("errors.cancelled"));
            break;
        }
        let tpl = match store::get(tid)? {
            Some(t) => t,
            None => continue,
        };
        // Share group cancel flag with each template run (nested register would double-count).
        match run_one_with_flag(state, &tpl, cancelled.clone(), dry_run).await {
            Ok((rows, msg)) => {
                total_rows += rows;
                messages.push(format!("{}: {msg}", tpl.name));
            }
            Err(e) => {
                err = Some(format!("«{}»: {e}", tpl.name));
                break;
            }
        }
    }

    runs::unregister(&run_key);
    let at = Utc::now().to_rfc3339();
    if let Some(e) = err {
        let cancelled_msg = t("errors.cancelled");
        let status = if cancelled.load(Ordering::Relaxed) || e.contains(&cancelled_msg) {
            "cancelled".to_string()
        } else {
            format!("error: {e}")
        };
        let _ = store::set_group_last_run(&group.id, &status);
        runs::emit_progress("group", &group.id, Some(&status), false, Some(&at));
        Err(e)
    } else {
        let _ = store::set_group_last_run(&group.id, "ok");
        runs::emit_progress("group", &group.id, Some("ok"), false, Some(&at));
        Ok((total_rows, messages.join("; ")))
    }
}

async fn run_one_with_flag(
    state: &AppState,
    tpl: &crate::templates::Template,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    dry_run: bool,
) -> Result<(usize, String), String> {
    let started_at = Utc::now().to_rfc3339();
    runs::emit_progress("template", &tpl.id, Some("running"), true, Some(&started_at));
    let result = run_one_inner(state, tpl, cancelled.clone(), dry_run).await;
    let at = Utc::now().to_rfc3339();
    match &result {
        Ok(_) => runs::emit_progress("template", &tpl.id, Some("ok"), false, Some(&at)),
        Err(e) => {
            let cancelled_msg = t("errors.cancelled");
            let status = if cancelled.load(Ordering::Relaxed) || e.contains(&cancelled_msg) {
                "cancelled".to_string()
            } else {
                format!("error: {e}")
            };
            runs::emit_progress("template", &tpl.id, Some(&status), false, Some(&at));
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
