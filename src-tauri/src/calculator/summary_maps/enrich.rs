//! Загрузка листов в карты артикул→колонки + BuildAdExpensesByNm (паритет).

use crate::calculator::sheet_util::{
    cell_at, find_header_index, first_http_url, format_ru_num,
    header_row, try_parse_number, SheetRows,
};
use std::collections::HashMap;


use super::wb_load::RowMap;

pub fn cell(map: Option<&HashMap<String, String>>, keys: &[&str]) -> String {
    let Some(m) = map else {
        return String::new();
    };
    for k in keys {
        if let Some(v) = m.get(*k) {
            if !v.trim().is_empty() {
                return v.clone();
            }
        }
        for (hk, hv) in m {
            if hk.eq_ignore_ascii_case(k) && !hv.trim().is_empty() {
                return hv.clone();
            }
        }
    }
    String::new()
}

pub fn put(values: &mut HashMap<String, String>, key: &str, val: String) {
    if !val.trim().is_empty() {
        values.insert(key.into(), val);
    }
}

/// Первая непустая строка ячейки (ТН-ВЭД / сертификаты в Массиве — многострочные).
pub fn first_nonempty_line(raw: &str) -> String {
    raw.split('\n')
        .map(str::trim)
        .find(|s| !s.is_empty() && *s != "0")
        .unwrap_or("")
        .to_string()
}

pub fn apply_commission_from_map(values: &mut HashMap<String, String>, commissions: &RowMap) {
    let Some(subject_id) = values.get("subjectId").cloned().filter(|s| !s.trim().is_empty()) else {
        return;
    };
    let Some(row) = commissions.get(&subject_id).or_else(|| {
        commissions
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&subject_id))
            .map(|(_, v)| v)
    }) else {
        return;
    };
    let fbo = cell(
        Some(row),
        &["Склад WB (FBW) %", "Комиссия FBO", "paidStorageKgvp", "kgvpSupplier"],
    );
    let fbs = cell(
        Some(row),
        &["Маркетплейс (FBS) %", "Комиссия FBS", "kgvpMarketplace"],
    );
    if let Some(n) = try_parse_number(&fbo) {
        values.insert("commissionFbo".into(), format_ru_num(n));
    } else if !fbo.trim().is_empty() {
        values.insert("commissionFbo".into(), fbo);
    }
    if let Some(n) = try_parse_number(&fbs) {
        values.insert("commissionFbs".into(), format_ru_num(n));
    } else if !fbs.trim().is_empty() {
        values.insert("commissionFbs".into(), fbs);
    }
}

pub fn apply_logistics_from_box(
    values: &mut HashMap<String, String>,
    box_rows: Option<&SheetRows>,
    is_sgt: bool,
) {
    let Some(rows) = box_rows else {
        return;
    };
    if rows.len() < 2 {
        return;
    }
    let len = try_parse_number(values.get("dimLength").map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    let wid = try_parse_number(values.get("dimWidth").map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    let hei = try_parse_number(values.get("dimHeight").map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    if len <= 0.0 || wid <= 0.0 || hei <= 0.0 {
        return;
    }
    let volume = (len * wid * hei / 1000.0).max(0.001);
    values.insert("volumeLiters".into(), format_ru_num(volume));
    values.insert(
        "isSgt".into(),
        if is_sgt { "да" } else { "нет" }.into(),
    );

    let header = header_row(rows);
    let base_idx = find_header_index(
        &header,
        "Логистика FBS 1л, ₽",
        &["Логистика FBS 1л, ₽", "Логистика 1л, ₽", "boxDeliveryMarketplaceBase"],
    );
    let liter_idx = find_header_index(
        &header,
        "Логистика FBS доп. л, ₽",
        &["Логистика FBS доп. л, ₽", "Логистика доп. л, ₽", "boxDeliveryMarketplaceLiter"],
    );
    let warehouse_idx = find_header_index(&header, "Склад", &["Склад", "warehouseName"]);
    let cabinet_idx = find_header_index(&header, "Кабинет", &["Кабинет", "cabinet"]);
    if base_idx < 0 || liter_idx < 0 || warehouse_idx < 0 {
        return;
    }
    let cabinet = values.get("cabinet").cloned().unwrap_or_default();
    let mut tariff: Option<&Vec<String>> = None;
    let mut any: Option<&Vec<String>> = None;
    for row in rows.iter().skip(1) {
        let wh = cell_at(row, warehouse_idx);
        let has_sgt = wh.to_ascii_uppercase().contains("СГТ") || wh.contains("СГТ");
        let ok = if is_sgt {
            wh.eq_ignore_ascii_case("Свой склад СГТ РФ")
                || (has_sgt && wh.contains("Свой склад"))
        } else {
            wh.eq_ignore_ascii_case("Свой склад РФ")
                || (!has_sgt && wh.contains("Свой склад") && wh.contains("РФ"))
        };
        if !ok || try_parse_number(&cell_at(row, base_idx)).is_none() {
            continue;
        }
        if any.is_none() {
            any = Some(row);
        }
        if cabinet_idx >= 0 && !cabinet.trim().is_empty() {
            let row_cab = cell_at(row, cabinet_idx);
            if !row_cab.trim().is_empty() && !row_cab.eq_ignore_ascii_case(&cabinet) {
                continue;
            }
        }
        tariff = Some(row);
        break;
    }
    let Some(row) = tariff.or(any) else {
        return;
    };
    let base = try_parse_number(&cell_at(row, base_idx)).unwrap_or(0.0);
    let liter = try_parse_number(&cell_at(row, liter_idx)).unwrap_or(0.0);
    let logistics = base + (volume - 1.0).max(0.0) * liter;
    values.insert("logistics".into(), format_ru_num(logistics));
    values.insert("logisticsWarehouse".into(), cell_at(row, warehouse_idx));
    values.insert("logisticsBase".into(), format_ru_num(base));
    values.insert("logisticsLiter".into(), format_ru_num(liter));
}

pub fn photo_url(raw: &str) -> String {
    first_http_url(raw)
}

