//! Загрузка nmId/chrtId фильтров из листа-источника.

use crate::core::script_field::ScriptField;
use serde_json::Value;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::script::ScriptContext;
use super::parse::{
    flatten_col, param, parse_chrt_list, parse_i64, try_detect_ranges_by_header,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct CabinetFilter {
    pub nm_ids: HashSet<i64>,
    pub chrt_ids: HashSet<i64>,
}


/// Кабинеты + nmId + chrtId из Second (и для `wb_stock_fbo_report`).
pub async fn load_filters(
    ctx: &ScriptContext,
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
) -> Result<HashMap<String, CabinetFilter>, String> {
    let mut cabinets_range = param(ctx, "cabinets_source_range").unwrap_or_default();
    let mut nm_range = param(ctx, "nm_id_source_range").unwrap_or_default();
    let mut chrt_range = param(ctx, "stocks_skus_source_range").unwrap_or_default();

    if cabinets_range.is_empty() || nm_range.is_empty() || chrt_range.is_empty() {
        if let Some((c, n, h)) =
            try_detect_ranges_by_header(ctx, client, spreadsheet_id, sheet_name).await?
        {
            if cabinets_range.is_empty() {
                cabinets_range = c;
            }
            if nm_range.is_empty() {
                nm_range = n;
            }
            if chrt_range.is_empty() {
                chrt_range = h;
            }
        }
    }

    if cabinets_range.is_empty() || nm_range.is_empty() || chrt_range.is_empty() {
        return Err(
            "Укажите диапазоны кабинетов, nmID и chrtId (или колонки «Кабинет», «Артикул WB», «Размеры: chrtID» в источнике)."
                .into(),
        );
    }

    ctx.log.info(format!(
        "Диапазоны: cabinets={cabinets_range}, nmId={nm_range}, chrtId={chrt_range}"
    ));

    let sheet_q = sheet_name.replace('\'', "''");
    let cab_a1 = format!("'{sheet_q}'!{cabinets_range}");
    let nm_a1 = format!("'{sheet_q}'!{nm_range}");
    let chrt_a1 = format!("'{sheet_q}'!{chrt_range}");

    ctx.log.info("Чтение кабинетов / nmId / chrtId из источника (batchGet)…");
    let batches = client
        .read_values_batch(spreadsheet_id, &[cab_a1.as_str(), nm_a1.as_str(), chrt_a1.as_str()])
        .await
        .map_err(|e| format!("Чтение источника: {e}"))?;
    let cab_vals = flatten_col(batches.first().map(Vec::as_slice).unwrap_or(&[]));
    let nm_vals = flatten_col(batches.get(1).map(Vec::as_slice).unwrap_or(&[]));
    let chrt_vals = flatten_col(batches.get(2).map(Vec::as_slice).unwrap_or(&[]));
    ctx.log.info(format!(
        "Прочитано строк: cabinets={}, nmId={}, chrtId={}",
        cab_vals.len(),
        nm_vals.len(),
        chrt_vals.len()
    ));

    let max_len = cab_vals.len().max(nm_vals.len()).max(chrt_vals.len());
    let mut map: HashMap<String, CabinetFilter> = HashMap::new();
    let mut skipped = 0usize;
    for i in 0..max_len {
        let cabinet = cab_vals.get(i).map(|s| s.trim()).unwrap_or("").to_string();
        let nm_raw = nm_vals.get(i).map(|s| s.as_str()).unwrap_or("");
        let nm = parse_i64(nm_raw);
        let chrts = chrt_vals
            .get(i)
            .map(|s| parse_chrt_list(s))
            .unwrap_or_default();
        if cabinet.is_empty() && nm.is_none() && chrts.is_empty() {
            continue;
        }
        // нужны cabinet + nmId>0 + хотя бы один chrtId в ячейке.
        let Some(nm_id) = nm.filter(|n| *n > 0) else {
            skipped += 1;
            continue;
        };
        if cabinet.is_empty() || chrts.is_empty() {
            skipped += 1;
            continue;
        }
        let entry = map.entry(cabinet).or_insert_with(|| CabinetFilter {
            nm_ids: HashSet::new(),
            chrt_ids: HashSet::new(),
        });
        entry.nm_ids.insert(nm_id);
        entry.chrt_ids.extend(chrts);
    }
    ctx.log.info(format!(
        "Кабинетов={}, пропущено строк={skipped}",
        map.len()
    ));
    Ok(map)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("chrtId", true),
        field("warehouseId", true),
        field("warehouseName", true),
        field("regionName", false),
        field("quantity", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_stocks.{name}")),
        selected,
    }
}

pub fn build_header(fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| f.title.clone())
        .collect()
}

pub fn build_data_row(cabinet: &str, item: &Value, fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| cell(f.name.as_str(), cabinet, item))
        .collect()
}

fn cell(name: &str, cabinet: &str, item: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmId" => num(item, "nmId"),
        "chrtId" => num(item, "chrtId"),
        "warehouseId" => num(item, "warehouseId"),
        "warehouseName" => str(item, "warehouseName"),
        "regionName" => str(item, "regionName"),
        "quantity" => num(item, "quantity"),
        _ => String::new(),
    }
}

fn str(it: &Value, key: &str) -> String {
    it.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn num(it: &Value, key: &str) -> String {
    match it.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}
