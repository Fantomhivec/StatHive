//! Создание/обновление таблиц калькулятора + запуск скриптов.

use crate::app_state::AppState;
use crate::calculator::catalog::{self, TableDef};
use crate::calculator::export_advertising;
use crate::calculator::profile::{self, CalculatorProfile};
use crate::calculator::run_log;
use crate::calculator::script_extras;
use crate::calculator::sheet_util;
use crate::calculator::snapshot;
use crate::calculator::summary_builder;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use crate::core::sheet_dates;
use crate::templates::{self, store};
use chrono::Local;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn create_or_update_all(
    state: Arc<AppState>,
    marketplace_scope: Option<&str>,
) -> Result<Value, String> {
    // onLog: прогресс скриптов → журнал калькулятора + live emit.
    let _mirror = run_log::enable_app_log_mirror();
    let mut profile = profile::load();
    let cabinets = profile.selected_cabinets.clone();
    if cabinets.is_empty() {
        return Err("Выберите хотя бы один кабинет".into());
    }

    let scope = marketplace_scope
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let tables_mp = snapshot::resolve_tables_marketplace(&profile);
    let tables: Vec<&TableDef> = catalog::tables_for_marketplace(&tables_mp)
        .into_iter()
        .filter(|t| !catalog::is_table_manual(&profile.table_modes, t.key))
        .filter(|t| !catalog::is_summary_table(t.key))
        .filter(|t| {
            scope.is_empty()
                || scope == "both"
                || t.marketplace.eq_ignore_ascii_case(&scope)
        })
        .collect();

    if tables.is_empty() {
        run_log::calc_info(
            &state.log,
            "[Калькулятор] Нет таблиц в режиме «Авто» для выбранной площадки.",
        );
        return Ok(json!({
            "ok": true,
            "message": "Нет таблиц в режиме «Авто» для выбранной площадки.",
            "snapshot": snapshot::get_snapshot()
        }));
    }

    let log = &state.log;
    let gs = GoogleSheetsClient::from_user_oauth().await?;

    let missing: Vec<&TableDef> = tables
        .iter()
        .copied()
        .filter(|t| {
            profile
                .table_links
                .iter()
                .find(|(k, v)| k.eq_ignore_ascii_case(t.key) && !v.trim().is_empty())
                .is_none()
        })
        .collect();
    let create_only = !missing.is_empty();

    if create_only {
        run_log::calc_info(
            log,
            format!(
                "[Калькулятор] Создание недостающих таблиц: {}…",
                missing.len()
            ),
        );
        for table in &missing {
            ensure_spreadsheet_linked(&mut profile, table, &gs, log).await?;
        }
        // Один save после пачки create (не на каждую таблицу).
        persist_partial(&profile);
    } else {
        run_log::calc_info(log, "[Калькулятор] Обновление таблиц…");
    }

    let to_export: Vec<&TableDef> = if create_only { missing } else { tables };
    let mut errors = Vec::new();
    let mut dirty = false;

    // Независимые таблицы — чанками (как CABINET_PARALLELISM); своды после цикла.
    let parallelism = crate::scripts::common::CABINET_PARALLELISM.max(1);
    let owned_tables: Vec<TableDef> = to_export.iter().map(|t| **t).collect();
    for chunk in owned_tables.chunks(parallelism) {
        let futs: Vec<_> = chunk
            .iter()
            .copied()
            .map(|table| {
                let state = state.clone();
                let mut profile_clone = profile.clone();
                let cabinets = cabinets.clone();
                let gs = gs.clone();
                let log = log.clone();
                async move {
                    if table.script_key.trim().is_empty() {
                        return (table, Ok(()), profile_clone);
                    }
                    let r =
                        run_one_table(state, &mut profile_clone, &table, &cabinets, &gs, &log).await;
                    (table, r, profile_clone)
                }
            })
            .collect();
        for (table, result, profile_clone) in futures_util::future::join_all(futs).await {
            match result {
                Ok(()) => {
                    for (k, v) in profile_clone.template_ids_by_table_key {
                        profile.template_ids_by_table_key.entry(k).or_insert(v);
                    }
                    stamp_updated(&mut profile, table.key);
                    dirty = true;
                    run_log::calc_info(
                        log,
                        format!("[Калькулятор] Готово «{}»", table.label_tr()),
                    );
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", table.label_tr()));
                    run_log::calc_info(
                        log,
                        format!("[Калькулятор] Ошибка «{}»: {e}", table.label_tr()),
                    );
                }
            }
        }
    }
    if dirty {
        persist_partial(&profile);
    }

    let has_wb = to_export
        .iter()
        .any(|t| t.marketplace.eq_ignore_ascii_case("wb"));
    let has_oz = to_export
        .iter()
        .any(|t| t.marketplace.eq_ignore_ascii_case("ozon"));
    if has_wb || scope.is_empty() || scope == "both" || scope == "wb" {
        match summary_builder::rebuild_wb_summary(&mut profile, &gs, log).await {
            Ok(_) => {}
            Err(e) => {
                errors.push(format!("Карточка WB: {e}"));
                run_log::calc_info(log, format!("[Калькулятор] Ошибка свода: {e}"));
            }
        }
    }
    if has_oz || scope.is_empty() || scope == "both" || scope == "ozon" {
        match summary_builder::rebuild_oz_summary(&mut profile, &gs, log).await {
            Ok(_) => {}
            Err(e) => {
                errors.push(format!("Карточка Ozon: {e}"));
                run_log::calc_info(log, format!("[Калькулятор] Ошибка свода Ozon: {e}"));
            }
        }
    }

    profile::save(&profile)?;
    let message = if !errors.is_empty() {
        format!(
            "Часть выгрузок не выполнилась:\n{}",
            errors.join("\n")
        )
    } else if create_only {
        "Созданы недостающие таблицы, данные выгружены, свод обновлён.".into()
    } else {
        "Обновление таблиц завершено, свод обновлён.".into()
    };

    Ok(json!({
        "ok": true,
        "message": message,
        "snapshot": snapshot::get_snapshot()
    }))
}

pub async fn refresh_table(state: Arc<AppState>, table_key: &str) -> Result<Value, String> {
    let _mirror = run_log::enable_app_log_mirror();
    let mut profile = profile::load();
    let Some(table) = catalog::find_table(table_key) else {
        return Err(format!("Неизвестная таблица «{table_key}»"));
    };
    if catalog::is_table_manual(&profile.table_modes, table.key) {
        return Ok(json!({
            "ok": true,
            "message": format!("«{}» в ручном режиме — скрипт не запускается.", table.label_tr()),
            "snapshot": snapshot::get_snapshot()
        }));
    }

    let cabinets = profile.selected_cabinets.clone();
    if cabinets.is_empty() && !table.script_key.is_empty() {
        return Err("Выберите хотя бы один кабинет".into());
    }

    let log = &state.log;
    let gs = GoogleSheetsClient::from_user_oauth().await?;
    ensure_spreadsheet_linked(&mut profile, table, &gs, log).await?;

    if catalog::is_summary_table(table.key) {
        if table.marketplace.eq_ignore_ascii_case("ozon") {
            summary_builder::rebuild_oz_summary(&mut profile, &gs, log).await?;
            profile::save(&profile)?;
            return Ok(json!({
                "ok": true,
                "message": "Свод «Карточка Ozon» обновлён.",
                "snapshot": snapshot::get_snapshot()
            }));
        }
        summary_builder::rebuild_wb_summary(&mut profile, &gs, log).await?;
        profile::save(&profile)?;
        return Ok(json!({
            "ok": true,
            "message": "Свод «Карточка WB» обновлён.",
            "snapshot": snapshot::get_snapshot()
        }));
    }

    if !table.script_key.is_empty() {
        run_one_table(state.clone(), &mut profile, table, &cabinets, &gs, log).await?;
    }
    stamp_updated(&mut profile, table.key);
    if table.marketplace.eq_ignore_ascii_case("wb") {
        let _ = summary_builder::rebuild_wb_summary(&mut profile, &gs, log).await;
    } else if table.marketplace.eq_ignore_ascii_case("ozon") {
        let _ = summary_builder::rebuild_oz_summary(&mut profile, &gs, log).await;
    }
    profile::save(&profile)?;
    Ok(json!({
        "ok": true,
        "message": format!("Обновление «{}» завершено.", table.label_tr()),
        "snapshot": snapshot::get_snapshot()
    }))
}

pub(crate) async fn ensure_spreadsheet_linked(
    profile: &mut CalculatorProfile,
    table: &TableDef,
    gs: &GoogleSheetsClient,
    log: &AppLog,
) -> Result<(), String> {
    if profile
        .table_links
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case(table.key) && !v.trim().is_empty())
    {
        return Ok(());
    }

    let folder_id = match profile.folder_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => id.to_string(),
        None => {
            let id = gs.ensure_folder("Калькулятор Universal").await?;
            profile.folder_id = Some(id.clone());
            id
        }
    };

    let drive_title = catalog::drive_title_for(table);
    let spreadsheet_id = if table.key.eq_ignore_ascii_case("Stocks") {
        let sid = gs
            .create_spreadsheet_in_folder(&drive_title, &folder_id, "Остатки FBS")
            .await?;
        let _ = gs.ensure_sheet_exists(&sid, "Остатки FBO").await;
        sid
    } else if table.key.eq_ignore_ascii_case("Advertising") {
        let sid = gs
            .create_spreadsheet_in_folder(&drive_title, &folder_id, "Кампании")
            .await?;
        let _ = gs.ensure_sheet_exists(&sid, "Расходы").await;
        let _ = gs.ensure_sheet_exists(&sid, "Инфо").await;
        sid
    } else {
        gs.create_spreadsheet_in_folder(&drive_title, &folder_id, &table.sheet_name_tr())
            .await?
    };

    let url = format!("https://docs.google.com/spreadsheets/d/{spreadsheet_id}");
    profile.table_links.insert(table.key.to_string(), url);
    run_log::calc_info(log, format!("[Калькулятор] Создана таблица «{drive_title}»"));
    Ok(())
}

async fn run_one_table(
    state: Arc<AppState>,
    profile: &mut CalculatorProfile,
    table: &TableDef,
    cabinets: &[String],
    gs: &GoogleSheetsClient,
    log: &AppLog,
) -> Result<(), String> {
    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table.key))
        .map(|(_, v)| v.clone())
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| format!("Нет ссылки на «{}»", table.label_tr()))?;
    let sid = sheet_util::extract_spreadsheet_id(&url)
        .ok_or_else(|| "Не удалось разобрать spreadsheetId".to_string())?;

    if table.key.eq_ignore_ascii_case("Advertising") {
        let _ = gs.ensure_sheet_exists(&sid, "Кампании").await;
        let _ = gs.ensure_sheet_exists(&sid, "Инфо").await;
        let _ = gs.ensure_sheet_exists(&sid, "Расходы").await;
    }

    let is_ozon = table.marketplace.eq_ignore_ascii_case("ozon");
    let filtered: Vec<String> = cabinets
        .iter()
        .filter(|c| {
            let mp = profile
                .cabinet_marketplaces
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(c))
                .map(|(_, v)| v.as_str())
                .unwrap_or("wb");
            let mp_l = mp.to_ascii_lowercase();
            if is_ozon {
                mp_l.contains("ozon")
            } else {
                !mp_l.contains("ozon")
            }
        })
        .cloned()
        .collect();
    // Не подставлять кабинеты другой площадки (раньше пустой filter → все cabinets).
    if filtered.is_empty() {
        return Err(if is_ozon {
            "Нет выбранных кабинетов Ozon (OZON_KEY_*)".into()
        } else {
            "Нет выбранных кабинетов WB (API_KEY_*)".into()
        });
    }
    let cabs = filtered;
    let mp = if is_ozon { "ozon" } else { "wb" };

    let script = state
        .registry
        .get(table.script_key)
        .ok_or_else(|| format!("Скрипт не найден: {}", table.script_key))?;
    let fields = script.default_fields();

    let mut tpl = if let Some(id) = profile
        .template_ids_by_table_key
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table.key))
        .map(|(_, v)| v.clone())
    {
        store::get(&id)?.unwrap_or_else(|| {
            templates::Template::new_blank(
                table.script_key,
                &table.label_tr(),
                fields.clone(),
                mp,
                "Калькулятор",
            )
        })
    } else {
        let mut t = templates::Template::new_blank(
            table.script_key,
            &table.label_tr(),
            fields,
            mp,
            "Калькулятор",
        );
        t.name = format!("[Калькулятор] {}", table.label_tr());
        t
    };

    tpl.name = format!("[Калькулятор] {}", table.label_tr());
    tpl.script_key = table.script_key.to_string();
    tpl.spreadsheet_id = sid.clone();
    tpl.sheet_name = table.sheet_name_tr();
    tpl.cabinets = cabs.clone();
    tpl.marketplace = mp.into();
    tpl.tab_name = "Калькулятор".into();
    if tpl.selected_fields.is_empty() {
        tpl.selected_fields = script.default_fields();
    }
    script_extras::apply_calculator_script_extras(
        profile,
        table,
        &mut tpl,
        &script.default_fields(),
        log,
    );
    wire_stocks_array_source(profile, table, &mut tpl, log);

    let saved = store::upsert(tpl)?;
    profile
        .template_ids_by_table_key
        .insert(table.key.to_string(), saved.id.clone());

    run_log::calc_info(log, format!("[Калькулятор] Выгрузка «{}»…", table.label_tr()));
    templates::run::run_one_template(state.as_ref(), &saved, false).await?;

    if table.key.eq_ignore_ascii_case("Advertising") {
        export_advertising::ensure_and_run_companions(state, profile, &sid, &cabs, log).await?;
    }
    Ok(())
}

/// Остатки FBS: nmId/chrtId из таблицы «Массив» (SecondSpreadsheet).
fn wire_stocks_array_source(
    profile: &CalculatorProfile,
    table: &TableDef,
    tpl: &mut templates::Template,
    log: &AppLog,
) {
    if !table.key.eq_ignore_ascii_case("Stocks") {
        return;
    }
    let Some(array_url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Array"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        run_log::calc_info(
            log,
            "[Калькулятор] Остатки: нет ссылки на «Массив» — источник nmId/chrtId не задан.",
        );
        return;
    };
    let Some(array_id) = sheet_util::extract_spreadsheet_id(&array_url) else {
        run_log::calc_info(
            log,
            "[Калькулятор] Остатки: не удалось извлечь ID таблицы «Массив».",
        );
        return;
    };
    let array_sheet = catalog::find_table("Array")
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| "Массив универсал".into());
    tpl.second_spreadsheet_id = array_id.clone();
    tpl.second_sheet_name = array_sheet;
    // Диапазоны пустые → скрипт найдёт колонки по заголовкам.
    tpl.cabinets_source_range.clear();
    tpl.nm_id_source_range.clear();
    tpl.stocks_skus_source_range.clear();
    run_log::calc_info(
        log,
        format!("[Калькулятор] Остатки FBS: источник nmId/chrtId = Массив ({array_id})."),
    );
}

fn stamp_updated(profile: &mut CalculatorProfile, key: &str) {
    let now = sheet_dates::format_datetime(Local::now().naive_local());
    profile.last_updated.insert(key.to_string(), now);
}

fn persist_partial(profile: &CalculatorProfile) {
    let _ = profile::save(profile);
}
