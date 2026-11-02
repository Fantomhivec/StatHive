//! Заказы Ozon FBO+FBS по SKU: окна 14+14 дней (паритет BuildOzOrdersBySkuAsync).

use crate::calculator::catalog;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    cell_at, find_header_index, header_row, is_valid_article, try_parse_number, SheetCache,
};
use crate::calculator::summary_maps::load_table_rows;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use crate::core::sheet_dates;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy)]
pub struct OzOrderCounts {
    pub cur: f64,
    pub prev: f64,
}

/// Сумма FBO+FBS по SKU: не cancelled; окна 14+14 от вчера.
pub async fn build_oz_orders_by_sku(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: &AppLog,
) -> HashMap<String, OzOrderCounts> {
    let mut result: HashMap<String, OzOrderCounts> = HashMap::new();
    let end = Local::now().date_naive() - Duration::days(1);
    let cur_start = end - Duration::days(13);
    let prev_end = cur_start - Duration::days(1);
    let prev_start = end - Duration::days(27);

    accumulate_oz_order_sheet(
        profile,
        gs,
        cache,
        "OzFboOrders",
        &["Дата создания отправления", "created_at"],
        &mut result,
        cur_start,
        end,
        prev_start,
        prev_end,
        log,
    )
    .await;
    accumulate_oz_order_sheet(
        profile,
        gs,
        cache,
        "OzFbsOrders",
        &["Дата начала обработки", "in_process_at"],
        &mut result,
        cur_start,
        end,
        prev_start,
        prev_end,
        log,
    )
    .await;

    result
}

async fn accumulate_oz_order_sheet(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    table_key: &str,
    date_headers: &[&str],
    result: &mut HashMap<String, OzOrderCounts>,
    cur_start: NaiveDate,
    cur_end: NaiveDate,
    prev_start: NaiveDate,
    prev_end: NaiveDate,
    log: &AppLog,
) {
    let Some(rows) = load_table_rows(profile, table_key, gs, cache, None, log).await else {
        return;
    };
    if rows.len() < 2 {
        return;
    }
    let header = header_row(&rows);
    let sku_idx = find_header_index(&header, "SKU товара", &["SKU товара", "product_sku", "SKU"]);
    let status_idx =
        find_header_index(&header, "Статус отправления", &["Статус отправления", "status"]);
    let date_idx = find_header_index(
        &header,
        date_headers.first().copied().unwrap_or(""),
        date_headers,
    );
    let qty_idx =
        find_header_index(&header, "Количество товара", &["Количество товара", "product_quantity"]);
    if sku_idx < 0 || date_idx < 0 {
        let label = catalog::find_table(table_key)
            .map(|t| t.label)
            .unwrap_or(table_key);
        run_log::calc_info(
            log,
            format!("[Калькулятор] «{label}»: нет колонок SKU/даты — заказы пропущены."),
        );
        return;
    }

    for row in rows.iter().skip(1) {
        let sku = cell_at(row, sku_idx);
        if !is_valid_article(&sku) {
            continue;
        }
        if status_idx >= 0 {
            let status = cell_at(row, status_idx);
            if status.eq_ignore_ascii_case("cancelled") {
                continue;
            }
        }
        let Some(day) = parse_sheet_date(&cell_at(row, date_idx)) else {
            continue;
        };
        let mut qty = 1.0;
        if qty_idx >= 0 {
            if let Some(q) = try_parse_number(&cell_at(row, qty_idx)) {
                if q > 0.0 {
                    qty = q;
                }
            }
        }
        let in_cur = day >= cur_start && day <= cur_end;
        let in_prev = day >= prev_start && day <= prev_end;
        if !in_cur && !in_prev {
            continue;
        }
        let entry = result.entry(sku.trim().to_string()).or_default();
        if in_cur {
            entry.cur += qty;
        }
        if in_prev {
            entry.prev += qty;
        }
    }
}

fn parse_sheet_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let formats = [
        sheet_dates::DATE_FORMAT,
        sheet_dates::DATETIME_FORMAT,
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%Y-%m-%d",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
    ];
    for f in formats {
        if let Ok(d) = NaiveDate::parse_from_str(s, f) {
            return Some(d);
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, f) {
            return Some(dt.date());
        }
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_local().date());
    }
    None
}
