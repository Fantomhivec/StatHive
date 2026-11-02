//! Загрузка листов в карты артикул→колонки + BuildAdExpensesByNm (паритет).

use crate::calculator::catalog;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    self, cell_at, find_header_index, format_ru_num, get_sheet_rows_res,
    header_row, is_valid_article, try_parse_number, ARTICLE_ALIASES, SheetCache, SheetRows,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use std::collections::{HashMap, HashSet};


use super::wb_load::{AdAgg, RowMap};

pub(crate) async fn load_sheet_map(
    profile: &CalculatorProfile,
    table_key: &str,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    article_headers: &[&str],
    sum_numeric_duplicates: bool,
    sheet_override: Option<&str>,
    log: &AppLog,
) -> RowMap {
    let mut result = RowMap::new();
    let Some(rows) = load_table_rows(profile, table_key, gs, cache, sheet_override, log).await
    else {
        return result;
    };
    if rows.len() < 2 {
        return result;
    }
    let header = header_row(&rows);
    let art_idx = find_header_index(&header, article_headers.first().copied().unwrap_or(""), article_headers);
    if art_idx < 0 {
        return result;
    }
    for row in rows.iter().skip(1) {
        let key = cell_at(row, art_idx);
        let subject_mode = article_headers.iter().any(|h| {
            h.eq_ignore_ascii_case("ID предмета")
                || h.eq_ignore_ascii_case("subjectID")
                || h.eq_ignore_ascii_case("subjectId")
        });
        if key.trim().is_empty() {
            continue;
        }
        if !subject_mode && !is_valid_article(&key) {
            continue;
        }
        let key = key.trim().to_string();
        let mut dict = HashMap::new();
        for (i, h) in header.iter().enumerate() {
            if h.trim().is_empty() {
                continue;
            }
            let v = cell_at(row, i as isize);
            if sum_numeric_duplicates {
                if let Some(existing) = result.get(&key).and_then(|m| m.get(h.as_str())) {
                    if let (Some(a), Some(b)) = (try_parse_number(existing), try_parse_number(&v)) {
                        dict.insert(h.clone(), format_ru_num(a + b));
                        continue;
                    }
                }
            }
            dict.insert(h.clone(), v);
        }
        if sum_numeric_duplicates {
            if let Some(prev) = result.get_mut(&key) {
                for (hk, hv) in dict {
                    if let Some(old) = prev.get(&hk) {
                        if let (Some(a), Some(b)) = (try_parse_number(old), try_parse_number(&hv)) {
                            prev.insert(hk, format_ru_num(a + b));
                            continue;
                        }
                    }
                    prev.entry(hk).or_insert(hv);
                }
            } else {
                result.insert(key, dict);
            }
        } else {
            result.entry(key).or_insert(dict);
        }
    }
    result
}

pub(crate) async fn load_sum_column_map(
    profile: &CalculatorProfile,
    table_key: &str,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    article_headers: &[&str],
    value_headers: &[&str],
    log: &AppLog,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(rows) = load_table_rows(profile, table_key, gs, cache, None, log).await else {
        return out;
    };
    if rows.len() < 2 {
        return out;
    }
    let header = header_row(&rows);
    let art_idx = find_header_index(&header, article_headers[0], article_headers);
    let val_idx = find_header_index(&header, value_headers[0], value_headers);
    if art_idx < 0 || val_idx < 0 {
        return out;
    }
    let mut sums: HashMap<String, f64> = HashMap::new();
    for row in rows.iter().skip(1) {
        let art = cell_at(row, art_idx);
        if !is_valid_article(&art) {
            continue;
        }
        if let Some(n) = try_parse_number(&cell_at(row, val_idx)) {
            *sums.entry(art.trim().to_string()).or_insert(0.0) += n;
        }
    }
    for (k, v) in sums {
        out.insert(k, format_ru_num(v));
    }
    out
}

pub(crate) async fn load_table_rows(
    profile: &CalculatorProfile,
    table_key: &str,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    sheet_override: Option<&str>,
    log: &AppLog,
) -> Option<SheetRows> {
    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())?;
    let sid = sheet_util::extract_spreadsheet_id(&url)?;
    let sheet = sheet_override
        .map(|s| s.to_string())
        .or_else(|| catalog::find_table(table_key).map(|t| t.sheet_name_tr()))?;
    match get_sheet_rows_res(gs, cache, &sid, &sheet).await {
        Ok(rows) => Some(rows),
        Err(e) => {
            let label = catalog::find_table(table_key)
                .map(|t| t.label_tr())
                .unwrap_or_else(|| table_key.to_string());
            run_log::calc_info(
                log,
                format!("[Калькулятор] «{label}»: ошибка чтения — {e}"),
            );
            None
        }
    }
}

pub(crate) async fn build_ad_expenses_by_nm(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: &AppLog,
) -> HashMap<String, AdAgg> {
    let mut result = HashMap::new();
    let Some(info) = load_table_rows(profile, "Advertising", gs, cache, Some("Инфо"), log).await
    else {
        return result;
    };
    if info.len() < 2 {
        return result;
    }
    let info_header = header_row(&info);
    let nm_idx = find_header_index(&info_header, "Артикул WB", ARTICLE_ALIASES);
    let advert_idx = find_header_index(
        &info_header,
        "ID кампании",
        &["ID кампании", "advertId", "AdvertId"],
    );
    if nm_idx < 0 || advert_idx < 0 {
        return result;
    }
    let mut nm_to_adverts: HashMap<String, HashSet<String>> = HashMap::new();
    for row in info.iter().skip(1) {
        let nm = cell_at(row, nm_idx);
        if !is_valid_article(&nm) {
            continue;
        }
        let id = cell_at(row, advert_idx);
        if id.trim().is_empty() || id == "0" {
            continue;
        }
        nm_to_adverts
            .entry(nm.trim().to_string())
            .or_default()
            .insert(id.trim().to_string());
    }

    let mut advert_sum: HashMap<String, f64> = HashMap::new();
    if let Some(exp) = load_table_rows(profile, "Advertising", gs, cache, Some("Расходы"), log).await
    {
        if exp.len() >= 2 {
            let exp_header = header_row(&exp);
            let exp_adv = find_header_index(
                &exp_header,
                "ID кампании",
                &["ID кампании", "advertId"],
            );
            let sum_idx =
                find_header_index(&exp_header, "Сумма", &["Сумма", "updSum", "Расходы", "Расход"]);
            if exp_adv >= 0 && sum_idx >= 0 {
                for row in exp.iter().skip(1) {
                    let id = cell_at(row, exp_adv);
                    if id.trim().is_empty() {
                        continue;
                    }
                    if let Some(n) = try_parse_number(&cell_at(row, sum_idx)) {
                        *advert_sum.entry(id.trim().to_string()).or_insert(0.0) += n;
                    }
                }
            }
        }
    }

    for (nm, ids) in nm_to_adverts {
        let mut sum = 0.0;
        for id in &ids {
            if let Some(s) = advert_sum.get(id) {
                sum += *s;
            }
        }
        result.insert(
            nm,
            AdAgg {
                has_ads: true,
                sum,
            },
        );
    }
    result
}
