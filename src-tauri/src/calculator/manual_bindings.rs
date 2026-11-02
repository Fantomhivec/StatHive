//! Ручные привязки колонок: resolve + карты артикул→значение для свода / Excel.

use crate::calculator::catalog::{self, MetricDef};
use crate::calculator::profile::{CalculatorColumnBinding, CalculatorProfile};
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    self, articles_match, cell_at, get_sheet_rows, header_row, is_valid_article,
    resolve_column_index, try_parse_number, ARTICLE_ALIASES, SheetCache,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use std::collections::HashMap;

/// metric_key → (артикул → значение).
pub type ManualMetricMaps = HashMap<String, HashMap<String, String>>;

pub fn resolve_binding(
    profile: &CalculatorProfile,
    metric: &MetricDef,
) -> Option<CalculatorColumnBinding> {
    if !catalog::is_table_manual(&profile.table_modes, metric.default_table_key) {
        return None;
    }
    let manual = profile
        .column_bindings
        .iter()
        .find(|b| b.metric_key.eq_ignore_ascii_case(metric.key));
    if let Some(m) = manual {
        if !m.column_header.trim().is_empty() && !m.spreadsheet_url.trim().is_empty() {
            return Some(CalculatorColumnBinding {
                metric_key: m.metric_key.clone(),
                spreadsheet_url: m.spreadsheet_url.clone(),
                sheet_name: if m.sheet_name.trim().is_empty() {
                    metric.default_sheet_name_tr()
                } else {
                    m.sheet_name.clone()
                },
                column_header: m.column_header.clone(),
                article_column_header: if m.article_column_header.trim().is_empty() {
                    metric.default_article_header_tr()
                } else {
                    m.article_column_header.clone()
                },
            });
        }
    }
    let linked = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(metric.default_table_key))
        .map(|(_, v)| v.clone())
        .filter(|u| !u.trim().is_empty());
    let url = manual
        .map(|m| m.spreadsheet_url.trim().to_string())
        .filter(|u| !u.is_empty())
        .or(linked)?;
    Some(CalculatorColumnBinding {
        metric_key: metric.key.into(),
        spreadsheet_url: url,
        sheet_name: manual
            .map(|m| m.sheet_name.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| metric.default_sheet_name_tr()),
        column_header: manual
            .map(|m| m.column_header.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| metric.default_column_header_tr()),
        article_column_header: manual
            .map(|m| m.article_column_header.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| metric.default_article_header_tr()),
    })
}

fn format_num(n: f64) -> String {
    if (n.fract()).abs() < 1e-9 {
        format!("{n:.0}")
    } else {
        format!("{n:.2}").replace('.', ",")
    }
}

fn map_get<'a>(map: &'a HashMap<String, String>, art: &str) -> Option<&'a String> {
    if let Some(v) = map.get(art) {
        return Some(v);
    }
    map.iter()
        .find(|(k, _)| articles_match(k, art))
        .map(|(_, v)| v)
}

fn is_sum_stock(key: &str) -> bool {
    matches!(
        key,
        "stockFbo" | "stockFbs" | "ozStock" | "ozFbsStock"
    )
}

fn warn_manual(
    log: Option<&AppLog>,
    warnings: Option<&mut Vec<String>>,
    metric_label: &str,
    detail: &str,
) {
    let msg = format!("Ручное поле «{metric_label}»: {detail}");
    if let Some(w) = warnings {
        w.push(msg.clone());
    }
    if let Some(l) = log {
        run_log::calc_info(l, format!("[Калькулятор] {msg}"));
    }
}

/// Загрузить все manual-метрики площадки (один read на binding).
pub async fn load_manual_metric_maps(
    profile: &CalculatorProfile,
    marketplace: &str,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: Option<&AppLog>,
    mut warnings: Option<&mut Vec<String>>,
) -> ManualMetricMaps {
    let mut out: ManualMetricMaps = HashMap::new();
    for metric in catalog::metrics_for_marketplace(marketplace) {
        if !catalog::is_table_manual(&profile.table_modes, metric.default_table_key) {
            continue;
        }
        let Some(binding) = resolve_binding(profile, metric) else {
            continue;
        };
        if binding.spreadsheet_url.trim().is_empty()
            || binding.sheet_name.trim().is_empty()
            || binding.column_header.trim().is_empty()
        {
            continue;
        }
        let Some(sid) = sheet_util::extract_spreadsheet_id(&binding.spreadsheet_url) else {
            continue;
        };
        let Some(rows) = get_sheet_rows(gs, cache, &sid, &binding.sheet_name).await else {
            warn_manual(log, warnings.as_deref_mut(), &metric.label_tr(), "лист недоступен.");
            continue;
        };
        if rows.len() < 2 {
            warn_manual(log, warnings.as_deref_mut(), &metric.label_tr(), "лист недоступен.");
            continue;
        }
        let header = header_row(&rows);
        let article_idx =
            resolve_column_index(&header, &binding.article_column_header, ARTICLE_ALIASES);
        let value_idx = resolve_column_index(&header, &binding.column_header, &[]);
        if article_idx < 0 || value_idx < 0 {
            warn_manual(
                log,
                warnings.as_deref_mut(),
                &metric.label_tr(),
                "не найдены колонки артикула/значения.",
            );
            continue;
        }
        let sum = is_sum_stock(metric.key);
        let mut map: HashMap<String, String> = HashMap::new();
        let mut sums: HashMap<String, f64> = HashMap::new();
        for row in rows.iter().skip(1) {
            let art = cell_at(row, article_idx);
            if !is_valid_article(&art) {
                continue;
            }
            let key = art.trim().to_string();
            let raw = cell_at(row, value_idx);
            if sum {
                if let Some(n) = try_parse_number(&raw) {
                    let next = sums.get(&key).copied().unwrap_or(0.0) + n;
                    sums.insert(key.clone(), next);
                    map.insert(key, format_num(next));
                }
            } else if !map.contains_key(&key) {
                let v = raw.trim();
                if !v.is_empty() {
                    map.insert(key, v.to_string());
                }
            }
        }
        if !map.is_empty() {
            out.insert(metric.key.to_string(), map);
        }
    }
    out
}

/// Вписать manual-значения в metric-map карточки (ключи `purchase` / `ozPurchase` / …).
pub fn apply_manual_to_values(
    values: &mut HashMap<String, String>,
    article: &str,
    maps: &ManualMetricMaps,
) {
    for (metric_key, art_map) in maps {
        let Some(v) = map_get(art_map, article) else {
            continue;
        };
        if v.trim().is_empty() {
            continue;
        }
        let owned = v.to_owned();
        if metric_key.eq_ignore_ascii_case("ozPurchase") {
            values.insert(metric_key.clone(), owned.clone());
            values.insert("purchase".into(), owned);
        } else {
            values.insert(metric_key.clone(), owned);
        }
    }
}

/// Вписать manual в строку свода по заголовкам колонок метрик.
pub fn apply_manual_to_headers(
    by_header: &mut HashMap<String, String>,
    article: &str,
    maps: &ManualMetricMaps,
    marketplace: &str,
) {
    for metric in catalog::metrics_for_marketplace(marketplace) {
        let Some(art_map) = maps.get(metric.key) else {
            continue;
        };
        let Some(v) = map_get(art_map, article) else {
            continue;
        };
        if v.trim().is_empty() {
            continue;
        }
        let header = if metric.default_column_header.trim().is_empty() {
            metric.label_tr()
        } else {
            metric.default_column_header_tr()
        };
        by_header.insert(header, v.clone());
    }
}

pub fn map_get_public<'a>(map: &'a HashMap<String, String>, art: &str) -> Option<&'a String> {
    map_get(map, art)
}
