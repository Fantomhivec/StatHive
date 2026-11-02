//! Lookup карточки WB / Ozon из связанных Google Sheets.

use crate::calculator::article_index;
use crate::calculator::card_reader_fill::{
    add_conversion, add_dynamics, apply_net_orders, fill_ad_expenses, fill_commissions,
    fill_metrics_from_catalog, fill_order_cost, overlay_manual_purchase_mp,
};
use crate::calculator::card_summary_row::fill_from_summary_row;
use crate::calculator::catalog::{self, WB_SOURCE_COLS};
use crate::calculator::logistics;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::profit::{evaluate_profit, format_profit_ru};
use crate::calculator::sheet_util::{
    self, articles_match, cell_at, find_header_index, first_http_url, format_ru_num,
    get_sheet_rows, header_row, is_valid_article, resolve_column_index, resolve_value_aliases,
    try_parse_number, ARTICLE_ALIASES, SUBJECT_ALIASES, SheetCache,
};
use crate::core::google_sheets::GoogleSheetsClient;
use serde_json::{json, Value};
use std::collections::HashMap;

const SUM_KEYS: &[&str] = &["stockFbo", "stockFbs", "ozStock", "ozFbsStock"];

pub async fn load_card(
    profile: &CalculatorProfile,
    article: &str,
    marketplace_filter: Option<&str>,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
) -> Value {
    let article = article.trim();
    if !is_valid_article(article) {
        return json!({
            "ok": false,
            "message": if article.is_empty() { "Введите артикул" } else { "Артикул/SKU «0» игнорируется" },
            "values": {},
            "profit": Value::Null
        });
    }

    let mut marketplace = marketplace_filter
        .unwrap_or(&profile.marketplace)
        .trim()
        .to_ascii_lowercase();
    if marketplace == "both" || marketplace == "auto" || marketplace.is_empty() {
        marketplace = "wb".into();
    }
    let is_ozon = marketplace.contains("ozon");

    let mut values: HashMap<String, String> = HashMap::new();
    let mut notes: Vec<String> = Vec::new();

    // RAM (прогрев) → иначе сеть. Без Массива / sources при hit свода.
    let from_summary =
        try_load_from_summary(profile, article, is_ozon, &mut values, cache, gs).await;
    if !from_summary {
        if is_ozon {
            fill_metrics_from_catalog(profile, article, "ozon", &mut values, cache, gs).await;
        } else {
            fill_metrics_from_wb_sources(profile, article, &mut values, cache, gs).await;
            enrich_from_array_extras(profile, article, &mut values, cache, gs).await;
            if value_empty(&values, "commissionFbo") || value_empty(&values, "commissionFbs") {
                fill_commissions(profile, &mut values, cache, gs, &mut notes).await;
            }
            if value_empty(&values, "logistics") {
                logistics::fill_logistics_wb(profile, &mut values, cache, gs, &mut notes).await;
            }
            if value_empty(&values, "adExpenses") || value_empty(&values, "hasAds") {
                fill_ad_expenses(profile, article, &mut values, cache, gs, &mut notes).await;
            }
            apply_net_orders(&mut values);
            if value_empty(&values, "orderCost") {
                fill_order_cost(&mut values);
            }
        }
        // Fallback без свода: закуп из ручной таблицы.
        overlay_manual_purchase_mp(profile, article, is_ozon, &mut values, cache, gs).await;
    }

    if !is_ozon {
        logistics::ensure_wb_logistics_breakdown(profile, &mut values, cache, gs, &mut notes).await;
    }

    if is_ozon {
        add_dynamics(&mut values, "orders", "ordersPrev", "ordersDelta");
    } else {
        add_dynamics(&mut values, "clicks", "clicksPrev", "clicksDelta");
        add_dynamics(&mut values, "cart", "cartPrev", "cartDelta");
        add_dynamics(&mut values, "orders", "ordersPrev", "ordersDelta");
        add_dynamics(&mut values, "sales", "salesPrev", "salesDelta");
        add_conversion(&mut values, "cart", "clicks", "clickToCart");
        add_conversion(&mut values, "orders", "clicks", "clickToOrder");
        add_conversion(&mut values, "orders", "cart", "cartToOrder");
    }

    if let Some(p) = values.get("ozPurchase").cloned() {
        if !p.trim().is_empty() {
            values.insert("purchase".into(), p);
        }
    }

    let formula = if is_ozon {
        &profile.ozon_profit_formula
    } else {
        &profile.profit_formula
    };
    let profit = evaluate_profit(formula, &values, profile, is_ozon);
    let ok = !values.is_empty();
    let message = if ok {
        notes.join(" ")
    } else {
        "Артикул не найден в подключённых таблицах".into()
    };
    let logistics_formula = if is_ozon {
        "Ozon: (магистраль min+max)/2 + FBS последняя миля + КГТ(бонус если вес/сумма сторон/объём)"
    } else {
        "FBS: СГТ→«Свой склад СГТ РФ» иначе «Свой склад РФ»; V=L*W*H/1000; cost=baseFbs+max(V-1,0)*literFbs"
    };

    json!({
        "ok": ok,
        "message": message,
        "article": article,
        "marketplace": if is_ozon { "ozon" } else { "wb" },
        "values": values,
        "profit": profit,
        "profitFormatted": profit.map(format_profit_ru),
        "logisticsFormula": logistics_formula
    })
}

fn value_empty(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| v.trim().is_empty() || v.trim() == "—")
        .unwrap_or(true)
}
async fn try_load_from_summary(
    profile: &CalculatorProfile,
    article: &str,
    is_ozon: bool,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
) -> bool {
    let table_key = if is_ozon {
        catalog::OZ_SUMMARY_KEY
    } else {
        catalog::WB_SUMMARY_KEY
    };
    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return false;
    };
    let Some(def) = catalog::find_table(table_key) else {
        return false;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        return false;
    };
    // Сначала RAM (площадка + номер строки после прогрева).
    if let Some((mem_oz, rows, idx, _sheet_row, _)) = article_index::memory_hit(article) {
        if mem_oz == is_ozon {
            let header = header_row(&rows);
            fill_from_summary_row(is_ozon, &header, &rows[idx], values);
            return !values.is_empty();
        }
    }
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &def.sheet_name_tr()).await else {
        return false;
    };
    if rows.len() < 2 {
        return false;
    }
    let header = header_row(&rows);
    let art_header = if is_ozon {
        crate::core::i18n::t("calc.headers.oz.sku")
    } else {
        crate::core::i18n::t("calc.headers.wb.article")
    };
    let article_idx = resolve_column_index(&header, &art_header, ARTICLE_ALIASES);
    if article_idx < 0 {
        return false;
    }
    let match_row = rows
        .iter()
        .skip(1)
        .find(|r| articles_match(&cell_at(r, article_idx), article));
    let Some(match_row) = match_row else {
        return false;
    };
    fill_from_summary_row(is_ozon, &header, match_row, values);
    !values.is_empty()
}

async fn fill_metrics_from_wb_sources(
    profile: &CalculatorProfile,
    article: &str,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
) {
    for src in WB_SOURCE_COLS {
        let key = src.key;
        let table_key = src.table_key;
        let sheet = src.sheet_name_tr();
        let col = src.column_tr();
        let art = src.article_tr();
        let Some(url) = profile
            .table_links
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
            .map(|(_, v)| v.trim().to_string())
            .filter(|u| !u.is_empty())
        else {
            continue;
        };
        let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
            continue;
        };
        let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
            continue;
        };
        if rows.len() < 2 {
            continue;
        }
        let header = header_row(&rows);
        let article_idx = resolve_column_index(&header, &art, ARTICLE_ALIASES);
        let value_aliases = if key == "photo" {
            vec!["photo1", "Фото", "photos"]
        } else {
            resolve_value_aliases(&col)
        };
        let value_idx = resolve_column_index(&header, &col, &value_aliases);
        if article_idx < 0 || value_idx < 0 {
            continue;
        }
        let sum_values = SUM_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key));
        let mut sum = 0.0;
        let mut found = false;
        let mut first = String::new();
        for row in rows.iter().skip(1) {
            if !articles_match(&cell_at(row, article_idx), article) {
                continue;
            }
            let raw = cell_at(row, value_idx);
            if !found {
                first = raw.clone();
                found = true;
            }
            if sum_values {
                if let Some(n) = try_parse_number(&raw) {
                    sum += n;
                }
            } else {
                break;
            }
        }
        if !found {
            continue;
        }
        let mut value = if sum_values {
            format_ru_num(sum)
        } else {
            first
        };
        if key == "photo" {
            value = first_http_url(&value);
        }
        values.insert(key.to_string(), value);
    }
}

async fn enrich_from_array_extras(
    profile: &CalculatorProfile,
    article: &str,
    values: &mut HashMap<String, String>,
    cache: &mut SheetCache,
    gs: &GoogleSheetsClient,
) {
    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Array"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        return;
    };
    let sheet = catalog::find_table("Array")
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| crate::core::i18n::t("calc.tables.Array.sheetName"));
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
        return;
    };
    if rows.len() < 2 {
        return;
    }
    let header = header_row(&rows);
    let article_idx = find_header_index(&header, "Артикул WB", ARTICLE_ALIASES);
    if article_idx < 0 {
        return;
    }
    let Some(row) = rows
        .iter()
        .skip(1)
        .find(|r| articles_match(&cell_at(r, article_idx), article))
    else {
        return;
    };

    let mut take = |key: &str, header_name: &str, aliases: &[&str]| {
        if values
            .get(key)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            return;
        }
        let idx = find_header_index(&header, header_name, aliases);
        if idx < 0 {
            return;
        }
        let raw = cell_at(row, idx);
        if !raw.trim().is_empty() {
            values.insert(key.to_string(), raw);
        }
    };

    take("subjectId", "ID предмета", SUBJECT_ALIASES);
    take("dimLength", "Длина, см", &resolve_value_aliases("Длина, см"));
    take("dimWidth", "Ширина, см", &resolve_value_aliases("Ширина, см"));
    take("dimHeight", "Высота, см", &resolve_value_aliases("Высота, см"));
    take(
        "dimWeight",
        "Вес, кг",
        &["Вес, кг", "dimensions.weightBrutto", "weightBrutto", "Вес"],
    );
    let mut take_first_line = |key: &str, header_name: &str, aliases: &[&str]| {
        if values
            .get(key)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            return;
        }
        let idx = find_header_index(&header, header_name, aliases);
        if idx < 0 {
            return;
        }
        let raw = cell_at(row, idx);
        let first = raw
            .split('\n')
            .map(str::trim)
            .find(|s| !s.is_empty());
        if let Some(v) = first {
            values.insert(key.to_string(), v.to_string());
        }
    };

    // EnrichFromArrayExtras: ТН-ВЭД + сертификаты (первая строка ячейки).
    take_first_line("tnved", "ТН-ВЭД", &["ТН-ВЭД", "tnved"]);
    take_first_line(
        "docNumber",
        "Номер сертификата/декларации",
        &["Номер сертификата/декларации", "doc_number", "cert_number"],
    );
    take_first_line(
        "docStart",
        "Дата регистрации сертификата/декларации",
        &[
            "Дата регистрации сертификата/декларации",
            "cert_reg_date",
        ],
    );
    take_first_line(
        "docEnd",
        "Дата окончания действия сертификата/декларации",
        &[
            "Дата окончания действия сертификата/декларации",
            "cert_expire_date",
        ],
    );
    take_first_line(
        "docCheckStatus",
        "Статус проверки",
        &["Статус проверки", "cert_check_status"],
    );
}

