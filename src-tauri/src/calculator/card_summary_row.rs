//! Заполнение `values` из одной строки свода (RAM hit или сеть).

use crate::calculator::catalog;
use crate::calculator::sheet_util::{
    cell_at, find_header_index, first_http_url, resolve_column_index, resolve_value_aliases,
};
use std::collections::HashMap;

pub fn fill_from_summary_row(
    is_ozon: bool,
    header: &[String],
    match_row: &[String],
    values: &mut HashMap<String, String>,
) {
    let mp = if is_ozon { "ozon" } else { "wb" };
    for metric in catalog::metrics_for_marketplace(mp) {
        let col_header = metric.default_column_header_tr();
        let aliases = resolve_value_aliases(&col_header);
        let col_idx = resolve_column_index(header, &col_header, &aliases);
        if col_idx < 0 {
            continue;
        }
        let mut raw = cell_at(match_row, col_idx);
        if raw.trim().is_empty() {
            continue;
        }
        if metric.key.eq_ignore_ascii_case("photo") {
            raw = first_http_url(&raw);
        }
        values.insert(metric.key.to_string(), raw);
    }
    for (col, key) in [
        ("Длина", "dimLength"),
        ("Ширина", "dimWidth"),
        ("Высота", "dimHeight"),
        ("Вес", "dimWeight"),
        ("Объём", "volumeLiters"),
        ("СГТ", "isSgt"),
        ("Склад FBS", "logisticsWarehouse"),
        ("База FBS 1л", "logisticsBase"),
        ("Доп. л FBS", "logisticsLiter"),
        ("ТН-ВЭД", "tnved"),
        ("Номер сертификата/декларации", "docNumber"),
        ("Дата регистрации сертификата/декларации", "docStart"),
        ("Дата окончания действия сертификата/декларации", "docEnd"),
        ("Статус проверки", "docCheckStatus"),
    ] {
        let idx = find_header_index(header, col, &[]);
        if idx < 0 {
            continue;
        }
        let raw = cell_at(match_row, idx);
        let first = raw
            .split('\n')
            .map(str::trim)
            .find(|s| !s.is_empty() && *s != "0");
        if let Some(v) = first {
            values.insert(key.into(), v.to_string());
        }
    }
}
