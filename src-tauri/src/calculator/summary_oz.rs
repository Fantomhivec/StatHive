//! Сборка свода «Карточка Ozon» (паритет BuildOzSummaryAsync).

use crate::calculator::catalog::{self, OZ_SUMMARY_KEY};
use crate::calculator::export_service;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{self, format_ru_num, is_valid_article, try_parse_number, SheetCache};
use crate::calculator::manual_bindings::{self, apply_manual_to_headers};
use crate::calculator::summary_maps::{cell, load_sheet_map};
use crate::calculator::summary_oz_orders::build_oz_orders_by_sku;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use crate::core::sheet_dates;
use chrono::Local;
use std::collections::HashSet;

const OZ_SKU_HEADERS: &[&str] = &["SKU (ID товара OZON)", "SKU"];
const OZ_SKU_HEADERS_REPORT: &[&str] = &["SKU", "SKU (ID товара OZON)"];

use crate::calculator::summary_oz_row::{build_oz_row, convert_oz_package_to_cm_kg, first_nonempty};
pub async fn rebuild_oz_summary(
    profile: &mut CalculatorProfile,
    gs: &GoogleSheetsClient,
    log: &AppLog,
) -> Result<usize, String> {
    let Some(table) = catalog::find_table(OZ_SUMMARY_KEY) else {
        return Err("Каталог: нет OzSummary".into());
    };
    export_service::ensure_spreadsheet_linked(profile, table, gs, log).await?;

    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(OZ_SUMMARY_KEY))
        .map(|(_, v)| v.clone())
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| "Нет ссылки на «Карточка Ozon»".to_string())?;
    let sid = sheet_util::extract_spreadsheet_id(&url)
        .ok_or_else(|| "Не удалось разобрать spreadsheetId свода Ozon".to_string())?;

    run_log::calc_info(log, "[Калькулятор] Сборка «Карточка Ozon»…");
    let mut cache = SheetCache::new();
    run_log::calc_info(log, "[Калькулятор] Карточка Ozon: загрузка исходных листов…");

    let (
        (prices, c_prices),
        (stocks, c_stocks),
        (report, c_report),
        (attrs, c_attrs),
        (purchases, c_purchases),
    ) = tokio::join!(
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "OzPrices",
                gs,
                &mut c,
                OZ_SKU_HEADERS,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "OzStocks",
                gs,
                &mut c,
                OZ_SKU_HEADERS,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "OzProductReport",
                gs,
                &mut c,
                OZ_SKU_HEADERS_REPORT,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "OzAttributes",
                gs,
                &mut c,
                OZ_SKU_HEADERS,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "OzPurchases",
                gs,
                &mut c,
                OZ_SKU_HEADERS_REPORT,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
    );

    for c in [c_prices, c_stocks, c_report, c_attrs, c_purchases] {
        crate::calculator::summary_maps::merge_sheet_cache(&mut cache, c);
    }

    let oz_orders = build_oz_orders_by_sku(profile, gs, &mut cache, log).await;
    let manual_maps = manual_bindings::load_manual_metric_maps(
        profile,
        "ozon",
        gs,
        &mut cache,
        Some(log),
        None,
    )
    .await;
    if !manual_maps.is_empty() {
        run_log::calc_info(
            log,
            format!(
                "[Калькулятор] Карточка Ozon: ручные поля — {} метрик.",
                manual_maps.len()
            ),
        );
    }

    run_log::calc_info(
        log,
        format!(
            "[Калькулятор] Карточка Ozon: карты готовы (Цены={}, Отчёт={}, Заказы SKU={}).",
            prices.len(),
            report.len(),
            oz_orders.len()
        ),
    );

    let mut sku_set: HashSet<String> = HashSet::new();
    for k in prices.keys() {
        sku_set.insert(k.clone());
    }
    for k in report.keys() {
        sku_set.insert(k.clone());
    }
    for k in stocks.keys() {
        sku_set.insert(k.clone());
    }
    for k in purchases.keys() {
        sku_set.insert(k.clone());
    }

    let mut sku_list: Vec<String> = sku_set
        .into_iter()
        .filter(|s| is_valid_article(s))
        .collect();
    sku_list.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    run_log::calc_info(
        log,
        format!(
            "[Калькулятор] Карточка Ozon: сборка {} SKU…",
            sku_list.len()
        ),
    );

    let mut rows: Vec<Vec<String>> = Vec::with_capacity(sku_list.len() + 1);
    let oz_headers = catalog::oz_headers();
    rows.push(oz_headers.clone());

    let mut done = 0usize;
    for sku in &sku_list {
        let p = prices.get(sku);
        let st = stocks.get(sku);
        let rp = report.get(sku);
        let at = attrs.get(sku);
        let pur = purchases.get(sku);

        let price_after_discount_ui = first_nonempty(&[
            cell(p, &["Цена"]),
            cell(rp, &["Цена на сайте, ₽"]),
        ]);
        let price_after_promo = first_nonempty(&[
            cell(p, &["Предельная цена с акциями продавца"]),
            price_after_discount_ui.clone(),
            cell(rp, &["Цена на сайте, ₽"]),
        ]);
        let discount = {
            let pa = try_parse_number(&price_after_discount_ui).unwrap_or(0.0);
            let pp = try_parse_number(&price_after_promo).unwrap_or(0.0);
            if pa > 0.0 {
                format_ru_num((pa - pp) / pa * 100.0)
            } else {
                String::new()
            }
        };

        let log_min = try_parse_number(&cell(p, &["FBS: магистраль от"])).unwrap_or(0.0);
        let log_max = try_parse_number(&cell(p, &["FBS: магистраль до"])).unwrap_or(0.0);
        let log_avg = if log_min > 0.0 || log_max > 0.0 {
            (log_min + log_max) / 2.0
        } else {
            0.0
        };
        let last_mile = try_parse_number(&cell(p, &["FBS: последняя миля"])).unwrap_or(0.0);

        let mut l = try_parse_number(&cell(at, &["Глубина"])).unwrap_or(0.0);
        let mut w = try_parse_number(&cell(at, &["Ширина упаковки"])).unwrap_or(0.0);
        let mut h = try_parse_number(&cell(at, &["Высота упаковки"])).unwrap_or(0.0);
        let mut weight = try_parse_number(&cell(at, &["Вес товара в упаковке"])).unwrap_or(0.0);
        convert_oz_package_to_cm_kg(
            &cell(at, &["Единица измерения габаритов"]),
            &cell(at, &["Единица измерения веса"]),
            &mut l,
            &mut w,
            &mut h,
            &mut weight,
        );
        let len = if l > 0.0 {
            format_ru_num(l)
        } else {
            String::new()
        };
        let wid = if w > 0.0 {
            format_ru_num(w)
        } else {
            String::new()
        };
        let hei = if h > 0.0 {
            format_ru_num(h)
        } else {
            String::new()
        };
        let wgt = if weight > 0.0 {
            format_ru_num(weight)
        } else {
            String::new()
        };

        let (vol_num, vol) = if l > 0.0 && w > 0.0 && h > 0.0 {
            let v = (l * w * h / 1000.0).max(0.001);
            (v, format_ru_num(v))
        } else {
            let raw = cell(rp, &["Объем товара, л"]);
            (try_parse_number(&raw).unwrap_or(0.0), raw)
        };

        let kgt_bonus = if profile.oz_kgt.is_kgt(l, w, h, weight, vol_num) {
            profile.oz_kgt.bonus
        } else {
            0.0
        };
        let logistics_total = log_avg + last_mile + kgt_bonus;

        let ord = oz_orders.get(sku);
        let orders_cur = ord
            .filter(|o| o.cur > 0.0)
            .map(|o| format_ru_num(o.cur))
            .unwrap_or_default();
        let orders_prev = ord
            .filter(|o| o.prev > 0.0)
            .map(|o| format_ru_num(o.prev))
            .unwrap_or_default();

        let mut by_header = build_oz_row(
            sku,
            p,
            st,
            rp,
            at,
            pur,
            &price_after_discount_ui,
            &discount,
            &price_after_promo,
            log_min,
            log_max,
            last_mile,
            kgt_bonus,
            logistics_total,
            &orders_cur,
            &orders_prev,
            &len,
            &wid,
            &hei,
            &wgt,
            &vol,
        );
        apply_manual_to_headers(&mut by_header, sku, &manual_maps, "ozon");
        let mut row = Vec::with_capacity(oz_headers.len());
        for h in &oz_headers {
            row.push(by_header.get(h).cloned().unwrap_or_default());
        }
        rows.push(row);
        done += 1;
        if done % 500 == 0 {
            run_log::calc_info(
                log,
                format!(
                    "[Калькулятор] Карточка Ozon: прогресс {done}/{}…",
                    sku_list.len()
                ),
            );
        }
    }

    run_log::calc_info(log, "[Калькулятор] Карточка Ozon: запись в Google Sheets…");
    gs.clear_and_write(&sid, &table.sheet_name_tr(), rows, true).await?;
    let now = sheet_dates::format_datetime(Local::now().naive_local());
    profile
        .last_updated
        .insert(OZ_SUMMARY_KEY.to_string(), now);
    run_log::calc_info(log, format!("[Калькулятор] «Карточка Ozon»: {done} строк."));
    crate::calculator::article_index::rebuild_after_oz_keys(profile, gs, sku_list, log).await;
    Ok(done)
}

