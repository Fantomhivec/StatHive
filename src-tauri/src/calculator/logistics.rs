//! Логистика WB из BoxTariffs + правила СГТ.
//! Ozon: логистика и КГТ-бонус считаются в `summary_oz` (из цен/атрибутов свода).

use crate::calculator::catalog;
use crate::core::i18n::t;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::sheet_util::{
    self, cell_at, find_header_index, format_ru_num, get_sheet_rows, resolve_value_aliases,
    try_parse_number, SheetCache,
};
use crate::core::google_sheets::GoogleSheetsClient;
use std::collections::HashMap;

fn warehouse_name_matches(warehouse_name: &str, want_sgt: bool) -> bool {
    let warehouse_sgt_rf = t("calc.logistics.warehouses.sgtRf");
    let warehouse_rf = t("calc.logistics.warehouses.rf");
    if warehouse_name.trim().is_empty() {
        return false;
    }
    let n = warehouse_name.trim();
    let has_sgt = n.to_ascii_uppercase().contains("СГТ")
        || n.contains("СГТ");
    if want_sgt {
        n.eq_ignore_ascii_case(&warehouse_sgt_rf)
            || (has_sgt && n.contains("Свой склад"))
    } else {
        n.eq_ignore_ascii_case(&warehouse_rf)
            || (!has_sgt && n.contains("Свой склад") && n.contains("РФ"))
    }
}

pub async fn fill_logistics_wb(
    profile: &CalculatorProfile,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
    notes: &mut Vec<String>,
) {
    let len = match try_parse_number(values.get("dimLength").map(|s| s.as_str()).unwrap_or("")) {
        Some(v) if v > 0.0 => v,
        _ => {
            notes.push("Логистика: нет габаритов в Массиве (Д×Ш×В).".into());
            return;
        }
    };
    let wid = match try_parse_number(values.get("dimWidth").map(|s| s.as_str()).unwrap_or("")) {
        Some(v) if v > 0.0 => v,
        _ => {
            notes.push("Логистика: нет габаритов в Массиве (Д×Ш×В).".into());
            return;
        }
    };
    let hei = match try_parse_number(values.get("dimHeight").map(|s| s.as_str()).unwrap_or("")) {
        Some(v) if v > 0.0 => v,
        _ => {
            notes.push("Логистика: нет габаритов в Массиве (Д×Ш×В).".into());
            return;
        }
    };
    let weight_kg =
        try_parse_number(values.get("dimWeight").map(|s| s.as_str()).unwrap_or("")).unwrap_or(0.0);

    let warehouse_sgt_rf = t("calc.logistics.warehouses.sgtRf");
    let warehouse_rf = t("calc.logistics.warehouses.rf");
    let is_sgt = profile.wb_kgt.is_sgt(len, wid, hei, weight_kg);
    let target = if is_sgt {
        warehouse_sgt_rf.clone()
    } else {
        warehouse_rf.clone()
    };
    values.insert("isSgt".into(), if is_sgt { "да" } else { "нет" }.into());

    let volume = (len * wid * hei / 1000.0).max(0.001);
    values.insert("volumeLiters".into(), format_ru_num(volume));

    let Some(box_url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("BoxTariffs"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        notes.push("Логистика: таблица «Короба» не подключена.".into());
        return;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&box_url) else {
        return;
    };
    let box_sheet = catalog::find_table("BoxTariffs")
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| t("calc.tables.BoxTariffs.sheetName"));

    let Some(rows) = get_sheet_rows(gs, cache, &sid, &box_sheet).await else {
        notes.push("Логистика: лист «Короба» пуст.".into());
        return;
    };
    if rows.len() < 2 {
        notes.push("Логистика: лист «Короба» пуст.".into());
        return;
    }
    let header = sheet_util::header_row(&rows);
    let base_aliases = resolve_value_aliases("Логистика FBS 1л, ₽");
    let liter_aliases = resolve_value_aliases("Логистика FBS доп. л, ₽");
    let base_idx = find_header_index(&header, "Логистика FBS 1л, ₽", &base_aliases);
    let liter_idx = find_header_index(&header, "Логистика FBS доп. л, ₽", &liter_aliases);
    let cabinet_label = t("calc.headers.wb.cabinet");
    let cabinet_idx =
        find_header_index(&header, &cabinet_label, &resolve_value_aliases(&cabinet_label));
    let warehouse_idx = find_header_index(&header, "Склад", &["Склад", "warehouseName"]);
    if base_idx < 0 || liter_idx < 0 {
        notes.push("Логистика: нет колонок «Логистика FBS 1л / доп. л».".into());
        return;
    }
    if warehouse_idx < 0 {
        notes.push("Логистика: нет колонки «Склад».".into());
        return;
    }

    let cabinet = values.get("cabinet").cloned().unwrap_or_default();
    let mut tariff_row: Option<&Vec<String>> = None;
    let mut tariff_any: Option<&Vec<String>> = None;

    for row in rows.iter().skip(1) {
        let wh = cell_at(row, warehouse_idx);
        if !warehouse_name_matches(&wh, is_sgt) {
            continue;
        }
        if try_parse_number(&cell_at(row, base_idx)).is_none() {
            continue;
        }
        if tariff_any.is_none() {
            tariff_any = Some(row);
        }
        if cabinet_idx >= 0 && !cabinet.trim().is_empty() {
            let row_cab = cell_at(row, cabinet_idx);
            if !row_cab.trim().is_empty() && !row_cab.eq_ignore_ascii_case(&cabinet) {
                continue;
            }
        }
        tariff_row = Some(row);
        break;
    }
    let tariff_row = tariff_row.or(tariff_any);
    let Some(tariff_row) = tariff_row else {
        notes.push(format!("Логистика: нет тарифа FBS для склада «{target}»."));
        return;
    };

    let base_rate = try_parse_number(&cell_at(tariff_row, base_idx)).unwrap_or(0.0);
    let liter_rate = try_parse_number(&cell_at(tariff_row, liter_idx)).unwrap_or(0.0);
    let logistics = base_rate + (volume - 1.0).max(0.0) * liter_rate;
    values.insert("logistics".into(), format_ru_num(logistics));
    values.insert("logisticsWarehouse".into(), cell_at(tariff_row, warehouse_idx));
    values.insert("logisticsBase".into(), format_ru_num(base_rate));
    values.insert("logisticsLiter".into(), format_ru_num(liter_rate));
}

fn logistics_part_empty(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| {
            let t = v.trim();
            t.is_empty() || t == "—" || t == "-"
        })
        .unwrap_or(true)
}

/// Свод хранит только итог. Если СГТ / склад / база / доп. литр пустые — добрать их из коробов,
/// не затирая уже посчитанную логистику.
pub async fn ensure_wb_logistics_breakdown(
    profile: &CalculatorProfile,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
    notes: &mut Vec<String>,
) {
    if !logistics_part_empty(values, "isSgt")
        && !logistics_part_empty(values, "logisticsWarehouse")
        && !logistics_part_empty(values, "logisticsBase")
        && !logistics_part_empty(values, "logisticsLiter")
    {
        return;
    }
    let kept = values
        .get("logistics")
        .cloned()
        .filter(|s| !s.trim().is_empty());
    let mut part_notes = Vec::new();
    fill_logistics_wb(profile, values, cache, gs, &mut part_notes).await;
    if let Some(kept) = kept {
        values.insert("logistics".into(), kept);
    } else {
        notes.extend(part_notes);
    }
}
