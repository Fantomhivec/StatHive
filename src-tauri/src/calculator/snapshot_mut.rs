//! Мутации профиля калькулятора (save / links / formula).

use crate::calculator::catalog::{self, tables_for_marketplace};
use crate::calculator::profile::{self, kgt_normalize, CalculatorColumnBinding};
use crate::calculator::sheet_util;
use crate::calculator::snapshot::{
    ensure_bundle_id, ensure_cabinet_marketplaces, ensure_table_modes, get_snapshot,
    resolve_tables_marketplace,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::templates::store as templates_store;
use serde_json::Value;
use std::collections::HashSet;

pub fn save_settings(p: &Value) -> Result<Value, String> {
    let mut profile = profile::load();
    ensure_bundle_id(&mut profile);

    if let Some(v) = p.get("calculatorName").and_then(|x| x.as_str()) {
        profile.calculator_name = v.to_string();
    }
    if let Some(v) = p.get("marketplace").and_then(|x| x.as_str()) {
        profile.marketplace = v.to_string();
    }
    let mut triggers_enabled_changed = false;
    if let Some(v) = p.get("triggersEnabled").and_then(|x| x.as_bool()) {
        if profile.triggers_enabled != v {
            triggers_enabled_changed = true;
        }
        profile.triggers_enabled = v;
    }
    macro_rules! f64_field {
        ($json:expr, $field:ident) => {
            if let Some(v) = p.get($json).and_then(|x| x.as_f64()) {
                profile.$field = v;
            }
        };
    }
    f64_field!("taxPercent", tax_percent);
    f64_field!("acquiringPercent", acquiring_percent);
    f64_field!("advPercent", adv_percent);
    f64_field!("extraExpensesPercent", extra_expenses_percent);
    f64_field!("ozonTaxPercent", ozon_tax_percent);
    f64_field!("ozonAcquiringPercent", ozon_acquiring_percent);
    f64_field!("ozonAdvPercent", ozon_adv_percent);
    f64_field!("ozonExtraExpensesPercent", ozon_extra_expenses_percent);

    if let Some(v) = p.get("profitFormula").and_then(|x| x.as_str()) {
        profile.profit_formula = v.to_string();
    }
    if let Some(v) = p.get("ozonProfitFormula").and_then(|x| x.as_str()) {
        profile.ozon_profit_formula = v.to_string();
    }

    if let Some(wb) = p.get("wbKgt").and_then(|x| x.as_object()) {
        let r = &mut profile.wb_kgt;
        if let Some(v) = wb.get("maxOneSideCm").and_then(|x| x.as_f64()) {
            r.max_one_side_cm = v;
        }
        if let Some(v) = wb.get("oneSideCmp").and_then(|x| x.as_str()) {
            r.one_side_cmp = kgt_normalize(v, "gte");
        }
        if let Some(v) = wb.get("maxSumSidesCm").and_then(|x| x.as_f64()) {
            r.max_sum_sides_cm = v;
        }
        if let Some(v) = wb.get("sumSidesCmp").and_then(|x| x.as_str()) {
            r.sum_sides_cmp = kgt_normalize(v, "gte");
        }
        if let Some(v) = wb.get("maxWeightKg").and_then(|x| x.as_f64()) {
            r.max_weight_kg = v;
        }
        if let Some(v) = wb.get("weightCmp").and_then(|x| x.as_str()) {
            r.weight_cmp = kgt_normalize(v, "gte");
        }
    }

    if let Some(oz) = p.get("ozKgt").and_then(|x| x.as_object()) {
        let r = &mut profile.oz_kgt;
        if let Some(v) = oz.get("maxSumSides").and_then(|x| x.as_f64()) {
            r.max_sum_sides = v;
        }
        if let Some(v) = oz.get("sumSidesCmp").and_then(|x| x.as_str()) {
            r.sum_sides_cmp = kgt_normalize(v, "gt");
        }
        if let Some(v) = oz.get("maxWeight").and_then(|x| x.as_f64()) {
            r.max_weight = v;
        }
        if let Some(v) = oz.get("weightCmp").and_then(|x| x.as_str()) {
            r.weight_cmp = kgt_normalize(v, "gt");
        }
        if let Some(v) = oz.get("maxVolume").and_then(|x| x.as_f64()) {
            r.max_volume = v;
        }
        if let Some(v) = oz.get("volumeCmp").and_then(|x| x.as_str()) {
            r.volume_cmp = kgt_normalize(v, "gt");
        }
        if let Some(v) = oz.get("bonus").and_then(|x| x.as_f64()) {
            r.bonus = v;
        }
    }

    if let Some(arr) = p.get("selectedCabinets").and_then(|x| x.as_array()) {
        profile.selected_cabinets = arr
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty())
            .collect();
        ensure_cabinet_marketplaces(&mut profile);
    }

    if let Some(obj) = p.get("tableLinks").and_then(|x| x.as_object()) {
        for (k, v) in obj {
            profile
                .table_links
                .insert(k.clone(), v.as_str().unwrap_or("").to_string());
        }
    }

    if let Some(obj) = p.get("tableModes").and_then(|x| x.as_object()) {
        for (k, v) in obj {
            let mode = v.as_str().unwrap_or("auto").trim().to_ascii_lowercase();
            profile.table_modes.insert(
                k.clone(),
                if mode == "manual" {
                    "manual".into()
                } else {
                    "auto".into()
                },
            );
        }
    }

    if let Some(arr) = p.get("columnBindings").and_then(|x| x.as_array()) {
        profile.column_bindings = arr
            .iter()
            .filter_map(|b| {
                let metric_key = b.get("metricKey")?.as_str()?.to_string();
                if metric_key.trim().is_empty() {
                    return None;
                }
                Some(CalculatorColumnBinding {
                    metric_key,
                    spreadsheet_url: b
                        .get("spreadsheetUrl")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    sheet_name: b
                        .get("sheetName")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    column_header: b
                        .get("columnHeader")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    article_column_header: b
                        .get("articleColumnHeader")
                        .and_then(|x| x.as_str())
                        .unwrap_or("Артикул ВБ")
                        .to_string(),
                })
            })
            .collect();
    }

    ensure_table_modes(&mut profile);
    profile::save(&profile)?;
    if triggers_enabled_changed {
        let _ = crate::calculator::triggers::sync_template_triggers_enabled(&profile);
    }
    Ok(get_snapshot())
}

pub fn edit_table_link(key: &str, url: &str) -> Result<Value, String> {
    if key.trim().is_empty() {
        return Err("Не указана таблица".into());
    }
    let mut profile = profile::load();
    profile
        .table_links
        .insert(key.to_string(), url.trim().to_string());
    profile::save(&profile)?;
    Ok(get_snapshot())
}

/// Удаляет авто-таблицы в Drive и снимает ссылки (ClearTableLinksAsync).
pub async fn clear_table_links() -> Result<Value, String> {
    let mut profile = profile::load();
    let tables_mp = resolve_tables_marketplace(&profile);
    let auto_keys: HashSet<String> = tables_for_marketplace(&tables_mp)
        .into_iter()
        .filter(|t| !catalog::is_table_manual(&profile.table_modes, t.key))
        .map(|t| t.key.to_string())
        .collect();

    let mut ids_to_delete: HashSet<String> = HashSet::new();
    for key in &auto_keys {
        if let Some(url) = profile
            .table_links
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
        {
            if let Some(id) = sheet_util::extract_spreadsheet_id(url) {
                ids_to_delete.insert(id);
            }
        }
    }

    let auto_template_ids: Vec<String> = profile
        .template_ids_by_table_key
        .iter()
        .filter(|(k, _)| auto_keys.iter().any(|ak| ak.eq_ignore_ascii_case(k)))
        .map(|(_, id)| id.clone())
        .collect();

    for tid in &auto_template_ids {
        if let Ok(Some(tpl)) = templates_store::get(tid) {
            let sid = tpl.spreadsheet_id.trim();
            if !sid.is_empty() {
                if let Some(id) = sheet_util::extract_spreadsheet_id(sid) {
                    ids_to_delete.insert(id);
                } else {
                    ids_to_delete.insert(sid.to_string());
                }
            }
        }
    }

    if !ids_to_delete.is_empty() {
        match GoogleSheetsClient::from_user_oauth().await {
            Ok(gs) => {
                for id in &ids_to_delete {
                    if let Err(e) = gs.delete_spreadsheet(id).await {
                        eprintln!("[Калькулятор] Не удалось удалить таблицу {id}: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("[Калькулятор] OAuth для удаления таблиц: {e}");
            }
        }
    }

    profile.table_links.retain(|k, _| {
        !auto_keys
            .iter()
            .any(|ak| ak.eq_ignore_ascii_case(k))
    });
    for k in &auto_keys {
        profile.last_updated.remove(k);
    }
    profile::save(&profile)?;

    for tid in &auto_template_ids {
        if let Ok(Some(mut tpl)) = templates_store::get(tid) {
            tpl.spreadsheet_id.clear();
            let _ = templates_store::upsert(tpl);
        }
    }

    Ok(get_snapshot())
}

pub fn append_profit_token(token: &str, formula_field: Option<&str>) -> Result<Value, String> {
    let mut profile = profile::load();
    let ozon = formula_field
        .unwrap_or("wb")
        .eq_ignore_ascii_case("ozon");
    let target = if ozon {
        &mut profile.ozon_profit_formula
    } else {
        &mut profile.profit_formula
    };
    if !target.is_empty() && !target.ends_with(' ') {
        target.push(' ');
    }
    target.push_str(token.trim());
    profile::save(&profile)?;
    Ok(get_snapshot())
}

pub fn clear_profit_formula(formula_field: Option<&str>) -> Result<Value, String> {
    let mut profile = profile::load();
    let ozon = formula_field
        .unwrap_or("wb")
        .eq_ignore_ascii_case("ozon");
    if ozon {
        profile.ozon_profit_formula.clear();
    } else {
        profile.profit_formula.clear();
    }
    profile::save(&profile)?;
    Ok(get_snapshot())
}

pub fn open_table_url(key: &str) -> Result<String, String> {
    let profile = profile::load();
    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
        .or_else(|| {
            profile
                .column_bindings
                .iter()
                .find(|b| !b.spreadsheet_url.trim().is_empty())
                .map(|b| b.spreadsheet_url.trim().to_string())
        })
        .ok_or_else(|| "Ссылка на таблицу не задана".to_string())?;
    Ok(url)
}
