//! Доборы комиссий / рекламы / закупа для CardReader.

use crate::calculator::catalog::{self, MetricDef};
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::sheet_util::{
    self, articles_match, cell_at, find_header_index, first_http_url, format_ru_num, get_sheet_rows,
    header_row, resolve_column_index, resolve_value_aliases, try_parse_number, ARTICLE_ALIASES,
    SUBJECT_ALIASES, SheetCache,
};
use crate::core::google_sheets::GoogleSheetsClient;
use std::collections::{HashMap, HashSet};

pub(crate) async fn fill_commissions(
    profile: &CalculatorProfile,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
    notes: &mut Vec<String>,
) {
    let Some(subject_id) = values.get("subjectId").cloned().filter(|s| !s.trim().is_empty()) else {
        notes.push("Комиссии: нет ID предмета в Массиве (перевыгрузите Массив).".into());
        return;
    };
    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Commissions"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        notes.push("Комиссии: таблица не подключена.".into());
        return;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        return;
    };
    let sheet = catalog::find_table("Commissions")
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| crate::core::i18n::t("calc.tables.Commissions.sheetName"));
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
        notes.push("Комиссии: лист пуст.".into());
        return;
    };
    if rows.len() < 2 {
        notes.push("Комиссии: лист пуст.".into());
        return;
    }
    let header = header_row(&rows);
    let subject_idx = find_header_index(&header, "ID предмета", SUBJECT_ALIASES);
    let fbo_idx = find_header_index(
        &header,
        "Склад WB (FBW) %",
        &resolve_value_aliases("Склад WB (FBW) %"),
    );
    let fbs_idx = find_header_index(
        &header,
        "Маркетплейс (FBS) %",
        &resolve_value_aliases("Маркетплейс (FBS) %"),
    );
    if subject_idx < 0 {
        notes.push("Комиссии: нет колонки ID предмета.".into());
        return;
    }
    let cabinet = values.get("cabinet").cloned().unwrap_or_default();
    let cabinet_idx = find_header_index(&header, "Кабинет", &resolve_value_aliases("Кабинет"));

    let mut match_row: Option<&Vec<String>> = None;
    for row in rows.iter().skip(1) {
        if !articles_match(&cell_at(row, subject_idx), &subject_id) {
            continue;
        }
        if cabinet_idx >= 0 && !cabinet.trim().is_empty() {
            let row_cab = cell_at(row, cabinet_idx);
            if !row_cab.trim().is_empty() && !row_cab.eq_ignore_ascii_case(&cabinet) {
                continue;
            }
        }
        match_row = Some(row);
        break;
    }
    if match_row.is_none() {
        match_row = rows
            .iter()
            .skip(1)
            .find(|row| articles_match(&cell_at(row, subject_idx), &subject_id));
    }
    let Some(match_row) = match_row else {
        notes.push(format!("Комиссии: предмет {subject_id} не найден."));
        return;
    };
    if fbo_idx >= 0 {
        let raw = cell_at(match_row, fbo_idx);
        if let Some(n) = try_parse_number(&raw) {
            values.insert("commissionFbo".into(), format_ru_num(n));
        } else if !raw.trim().is_empty() {
            values.insert("commissionFbo".into(), raw);
        }
    }
    if fbs_idx >= 0 {
        let raw = cell_at(match_row, fbs_idx);
        if let Some(n) = try_parse_number(&raw) {
            values.insert("commissionFbs".into(), format_ru_num(n));
        } else if !raw.trim().is_empty() {
            values.insert("commissionFbs".into(), raw);
        }
    }
}

pub(crate) async fn fill_ad_expenses(
    profile: &CalculatorProfile,
    article: &str,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
    notes: &mut Vec<String>,
) {
    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Advertising"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        return;
    };
    let Some(info_rows) = get_sheet_rows(gs, cache, &sid, "Инфо").await else {
        notes.push("Реклама: лист «Инфо» пуст (обновите «Реклама»).".into());
        return;
    };
    if info_rows.len() < 2 {
        notes.push("Реклама: лист «Инфо» пуст (обновите «Реклама»).".into());
        return;
    }
    let info_header = header_row(&info_rows);
    let nm_idx = find_header_index(&info_header, "Артикул WB", ARTICLE_ALIASES);
    let advert_idx = find_header_index(
        &info_header,
        "ID кампании",
        &["ID кампании", "advertId", "AdvertId", "campaignId"],
    );
    let info_cab_idx =
        find_header_index(&info_header, "Кабинет", &resolve_value_aliases("Кабинет"));
    if nm_idx < 0 || advert_idx < 0 {
        notes.push("Реклама: на «Инфо» нет колонок Артикул WB / ID кампании.".into());
        return;
    }
    let cabinet = values.get("cabinet").cloned().unwrap_or_default();
    let mut advert_ids: HashSet<String> = HashSet::new();
    for row in info_rows.iter().skip(1) {
        if !articles_match(&cell_at(row, nm_idx), article) {
            continue;
        }
        if info_cab_idx >= 0 && !cabinet.trim().is_empty() {
            let row_cab = cell_at(row, info_cab_idx);
            if !row_cab.trim().is_empty() && !row_cab.eq_ignore_ascii_case(&cabinet) {
                continue;
            }
        }
        let id = cell_at(row, advert_idx);
        if !id.trim().is_empty() && id != "0" {
            advert_ids.insert(id.trim().to_string());
        }
    }
    // Fallback без фильтра кабинета.
    if advert_ids.is_empty() && info_cab_idx >= 0 && !cabinet.trim().is_empty() {
        for row in info_rows.iter().skip(1) {
            if !articles_match(&cell_at(row, nm_idx), article) {
                continue;
            }
            let id = cell_at(row, advert_idx);
            if !id.trim().is_empty() && id != "0" {
                advert_ids.insert(id.trim().to_string());
            }
        }
    }

    let has_link = !advert_ids.is_empty();
    let mut total = 0.0;
    if has_link {
        let Some(exp_rows) = get_sheet_rows(gs, cache, &sid, "Расходы").await else {
            notes.push("Реклама: лист «Расходы» пуст (обновите «Реклама»).".into());
            values.insert("hasAds".into(), "да".into());
            values.insert("adExpenses".into(), "0".into());
            return;
        };
        if exp_rows.len() < 2 {
            notes.push("Реклама: лист «Расходы» пуст (обновите «Реклама»).".into());
        } else {
            let exp_header = header_row(&exp_rows);
            let exp_adv_idx = find_header_index(
                &exp_header,
                "ID кампании",
                &["ID кампании", "advertId", "AdvertId", "campaignId"],
            );
            let sum_idx = find_header_index(
                &exp_header,
                "Сумма",
                &["Сумма", "updSum", "Расходы", "Расход"],
            );
            let exp_cab_idx =
                find_header_index(&exp_header, "Кабинет", &resolve_value_aliases("Кабинет"));
            if exp_adv_idx < 0 || sum_idx < 0 {
                notes.push("Реклама: на «Расходы» нет колонок ID кампании / Сумма.".into());
            } else {
                for row in exp_rows.iter().skip(1) {
                    let id = cell_at(row, exp_adv_idx);
                    if !advert_ids.contains(id.trim()) {
                        continue;
                    }
                    if exp_cab_idx >= 0 && !cabinet.trim().is_empty() {
                        let row_cab = cell_at(row, exp_cab_idx);
                        if !row_cab.trim().is_empty() && !row_cab.eq_ignore_ascii_case(&cabinet) {
                            continue;
                        }
                    }
                    if let Some(n) = try_parse_number(&cell_at(row, sum_idx)) {
                        total += n;
                    }
                }
            }
        }
    }
    // : hasAds = есть связка на Инфо ИЛИ сумма расходов > 0.
    values.insert(
        "hasAds".into(),
        if has_link || total > 0.0 { "да" } else { "нет" }.into(),
    );
    values.insert("adExpenses".into(), format_ru_num(total));
}

pub(crate) fn apply_net_orders(values: &mut HashMap<String, String>) {
    apply_net_order_key(values, "orders", "cancels", "ordersGross");
    apply_net_order_key(values, "ordersPrev", "cancelsPrev", "ordersPrevGross");
}

fn apply_net_order_key(
    values: &mut HashMap<String, String>,
    orders_key: &str,
    cancels_key: &str,
    gross_key: &str,
) {
    if !values.contains_key(orders_key) && !values.contains_key(cancels_key) {
        return;
    }
    let orders = try_parse_number(values.get(orders_key).map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    let cancels = try_parse_number(values.get(cancels_key).map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    values.insert(gross_key.into(), format_ru_num(orders));
    values.insert(orders_key.into(), format_ru_num((orders - cancels).max(0.0)));
}

pub(crate) fn fill_order_cost(values: &mut HashMap<String, String>) {
    let ad = try_parse_number(values.get("adExpenses").map(|s| s.as_str()).unwrap_or(""))
        .unwrap_or(0.0);
    let orders =
        try_parse_number(values.get("orders").map(|s| s.as_str()).unwrap_or("")).unwrap_or(0.0);
    let cost = if orders > 0.0 {
        ad / orders
    } else if ad > 0.0 {
        ad
    } else {
        0.0
    };
    values.insert("orderCost".into(), format_ru_num(cost));
}

/// Ручной закуп WB (`purchase`) / Ozon (`ozPurchase`) поверх свода.
pub(crate) async fn overlay_manual_purchase_mp(
    profile: &CalculatorProfile,
    article: &str,
    is_ozon: bool,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
) {
    let key = if is_ozon { "ozPurchase" } else { "purchase" };
    let mp = if is_ozon { "ozon" } else { "wb" };
    let table_fallback = if is_ozon { "OzPurchases" } else { "Purchases" };
    let Some(metric) = catalog::metrics()
        .iter()
        .find(|m| m.marketplace == mp && m.key == key)
    else {
        return;
    };
    let url = profile
        .column_bindings
        .iter()
        .find(|b| b.metric_key.eq_ignore_ascii_case(key) && !b.spreadsheet_url.trim().is_empty())
        .map(|b| b.spreadsheet_url.clone())
        .or_else(|| {
            profile
                .table_links
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(table_fallback))
                .map(|(_, v)| v.clone())
        })
        .filter(|u| !u.trim().is_empty());
    let Some(url) = url else {
        return;
    };
    let binding = profile
        .column_bindings
        .iter()
        .find(|b| b.metric_key.eq_ignore_ascii_case(key));
    let default_sheet = metric.default_sheet_name_tr();
    let default_col = metric.default_column_header_tr();
    let default_art = metric.default_article_header_tr();
    let sheet = binding
        .map(|b| b.sheet_name.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(default_sheet);
    let col = binding
        .map(|b| b.column_header.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(default_col);
    let art = binding
        .map(|b| b.article_column_header.clone())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(default_art);
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        return;
    };
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
        return;
    };
    if rows.len() < 2 {
        return;
    }
    let header = header_row(&rows);
    let article_idx = resolve_column_index(&header, &art, ARTICLE_ALIASES);
    let value_idx = resolve_column_index(&header, &col, &[]);
    if article_idx < 0 || value_idx < 0 {
        return;
    }
    for row in rows.iter().skip(1) {
        if articles_match(&cell_at(row, article_idx), article) {
            let raw = cell_at(row, value_idx);
            if !raw.trim().is_empty() {
                values.insert(key.into(), raw.clone());
                values.insert("purchase".into(), raw);
            }
            break;
        }
    }
}

pub(crate) fn add_dynamics(values: &mut HashMap<String, String>, cur: &str, prev: &str, delta: &str) {
    let Some(c) = try_parse_number(values.get(cur).map(|s| s.as_str()).unwrap_or("")) else {
        return;
    };
    let Some(p) = try_parse_number(values.get(prev).map(|s| s.as_str()).unwrap_or("")) else {
        return;
    };
    if p.abs() < 0.000001 {
        values.insert(delta.into(), "—".into());
        return;
    }
    let pct = (c - p) / p * 100.0;
    let sign = if pct > 0.0 { "+" } else { "" };
    values.insert(delta.into(), format!("{sign}{:.1}%", pct).replace('.', ","));
}

pub(crate) fn add_conversion(values: &mut HashMap<String, String>, num: &str, den: &str, out: &str) {
    let Some(n) = try_parse_number(values.get(num).map(|s| s.as_str()).unwrap_or("")) else {
        return;
    };
    let Some(d) = try_parse_number(values.get(den).map(|s| s.as_str()).unwrap_or("")) else {
        values.insert(out.into(), "—".into());
        return;
    };
    if d.abs() < 0.000001 {
        values.insert(out.into(), "—".into());
        return;
    }
    values.insert(out.into(), format!("{}%", format_ru_num(n / d * 100.0)));
}

fn metric_value_empty(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| v.trim().is_empty() || v.trim() == "—")
        .unwrap_or(true)
}

/// Fallback Ozon: метрики каталога (свод / OzPurchases). Один проход строки на лист.
pub(crate) async fn fill_metrics_from_catalog(
    profile: &CalculatorProfile,
    article: &str,
    marketplace: &str,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
) {
    let mut by_sheet: HashMap<(&str, &str), Vec<&MetricDef>> = HashMap::new();
    for metric in catalog::metrics_for_marketplace(marketplace) {
        if !metric_value_empty(values, metric.key) {
            continue;
        }
        by_sheet
            .entry((metric.default_table_key, metric.default_sheet_name))
            .or_default()
            .push(metric);
    }

    for ((table_key, sheet_key), metrics) in by_sheet {
        let url = profile
            .table_links
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
            .map(|(_, v)| v.trim().to_string())
            .filter(|u| !u.is_empty());
        let Some(url) = url else {
            continue;
        };
        let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
            continue;
        };
        let sheet = crate::core::i18n::t(sheet_key);
        let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
            continue;
        };
        if rows.len() < 2 {
            continue;
        }
        let header = header_row(&rows);

        let mut by_art: HashMap<&str, Vec<&MetricDef>> = HashMap::new();
        for m in metrics {
            by_art
                .entry(m.default_article_header)
                .or_default()
                .push(m);
        }

        for (art_header_key, group) in by_art {
            let art_header = crate::core::i18n::t(art_header_key);
            let art_idx = resolve_column_index(&header, &art_header, ARTICLE_ALIASES);
            if art_idx < 0 {
                continue;
            }
            let Some(row) = rows
                .iter()
                .skip(1)
                .find(|r| articles_match(&cell_at(r, art_idx), article))
            else {
                continue;
            };
            for metric in group {
                if !metric_value_empty(values, metric.key) {
                    continue;
                }
                let col_header = metric.default_column_header_tr();
                let aliases = resolve_value_aliases(&col_header);
                let col_idx = resolve_column_index(&header, &col_header, &aliases);
                if col_idx < 0 {
                    continue;
                }
                let mut raw = cell_at(row, col_idx);
                if raw.trim().is_empty() {
                    continue;
                }
                if metric.key.eq_ignore_ascii_case("photo") {
                    raw = first_http_url(&raw);
                }
                values.insert(metric.key.to_string(), raw);
            }
        }
    }
}
