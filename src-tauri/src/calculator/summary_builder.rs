//! Сборка свода «Карточка WB» map-join (паритет).
//! Ozon: см. `summary_oz::rebuild_oz_summary`.

pub use crate::calculator::summary_oz::rebuild_oz_summary;

use crate::calculator::card_reader_fill::{apply_net_orders, fill_order_cost};
use crate::calculator::catalog::{self, WB_SUMMARY_KEY};
use crate::core::i18n::t;
use crate::calculator::export_service;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    self, cell_at, find_header_index, get_sheet_rows_res, header_row, is_valid_article,
    ARTICLE_ALIASES, SheetCache,
};
use crate::calculator::manual_bindings::{self, apply_manual_to_values};
use crate::calculator::summary_maps::{
    apply_commission_from_map, apply_logistics_from_box, cell, first_nonempty_line, load_wb_maps,
    photo_url, put,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use crate::core::sheet_dates;
use chrono::Local;
use std::collections::HashMap;

const VALUE_TO_HEADER: &[(&str, &str)] = &[
    ("cabinet", "calc.headers.wb.cabinet"),
    ("photo", "calc.headers.wb.photo"),
    ("name", "calc.headers.wb.name"),
    ("brand", "calc.headers.wb.brand"),
    ("vendorCode", "calc.headers.wb.vendorPst"),
    ("category", "calc.headers.wb.category"),
    ("stockFbo", "calc.headers.wb.stockFbo"),
    ("stockFbs", "calc.headers.wb.stockFbs"),
    ("clicks", "calc.headers.wb.clicks"),
    ("clicksPrev", "calc.headers.wb.clicksPrev"),
    ("cart", "calc.headers.wb.cart"),
    ("cartPrev", "calc.headers.wb.cartPrev"),
    ("wishlist", "calc.headers.wb.wishlist"),
    ("orders", "calc.headers.wb.orders"),
    ("ordersPrev", "calc.headers.wb.ordersPrev"),
    ("cancels", "calc.headers.wb.cancels"),
    ("cancelsPrev", "calc.headers.wb.cancelsPrev"),
    ("sales", "calc.headers.wb.sales"),
    ("salesPrev", "calc.headers.wb.salesPrev"),
    ("rating", "calc.headers.wb.rating"),
    ("feedbacks", "calc.headers.wb.feedbacks"),
    ("price", "calc.headers.wb.price"),
    ("discount", "calc.headers.wb.discount"),
    ("priceAfterDiscount", "calc.headers.wb.priceAfterDiscount"),
    ("commissionFbo", "calc.headers.wb.commissionFbo"),
    ("commissionFbs", "calc.headers.wb.commissionFbs"),
    ("purchase", "calc.headers.wb.purchase"),
    ("logistics", "calc.headers.wb.logistics"),
    ("hasAds", "calc.headers.wb.hasAdvertising"),
    ("adExpenses", "calc.headers.wb.adSpend2w"),
    ("orderCost", "calc.headers.wb.orderCost"),
    ("dimLength", "calc.headers.wb.length"),
    ("dimWidth", "calc.headers.wb.width"),
    ("dimHeight", "calc.headers.wb.height"),
    ("dimWeight", "calc.headers.wb.weight"),
    ("volumeLiters", "calc.headers.wb.volume"),
    ("isSgt", "calc.headers.wb.sgt"),
    ("logisticsWarehouse", "calc.headers.wb.logisticsWarehouse"),
    ("logisticsBase", "calc.headers.wb.logisticsBase"),
    ("logisticsLiter", "calc.headers.wb.logisticsLiter"),
    ("tnved", "calc.headers.wb.tnved"),
    ("docNumber", "calc.headers.wb.certNumber"),
    ("docStart", "calc.headers.wb.certRegDate"),
    ("docEnd", "calc.headers.wb.certEndDate"),
    ("docCheckStatus", "calc.headers.wb.certStatus"),
];

pub async fn rebuild_wb_summary(
    profile: &mut CalculatorProfile,
    gs: &GoogleSheetsClient,
    log: &AppLog,
) -> Result<usize, String> {
    let Some(table) = catalog::find_table(WB_SUMMARY_KEY) else {
        return Err("Каталог: нет WbSummary".into());
    };
    export_service::ensure_spreadsheet_linked(profile, table, gs, log).await?;

    let url = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(WB_SUMMARY_KEY))
        .map(|(_, v)| v.clone())
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| "Нет ссылки на «Карточка WB»".to_string())?;
    let sid = sheet_util::extract_spreadsheet_id(&url)
        .ok_or_else(|| "Не удалось разобрать spreadsheetId свода".to_string())?;

    run_log::calc_info(log, "[Калькулятор] Сборка «Карточка WB»…");
    let mut cache = SheetCache::new();
    let maps = load_wb_maps(profile, gs, &mut cache, log).await;
    let manual_maps =
        manual_bindings::load_manual_metric_maps(profile, "wb", gs, &mut cache, Some(log), None)
            .await;
    if !manual_maps.is_empty() {
        run_log::calc_info(
            log,
            format!(
                "[Калькулятор] Карточка WB: ручные поля — {} метрик.",
                manual_maps.len()
            ),
        );
    }
    if maps.array.is_empty() {
        // Диагностика: почему Массив «пуст» после успешной выгрузки.
        diagnose_empty_array(profile, gs, &mut cache, log).await;
        run_log::calc_info(
            log,
            "[Калькулятор] Карточка WB: Массив пуст или ссылка недоступна — свод не собран.",
        );
        return Ok(0);
    }

    let mut articles: Vec<String> = maps
        .array
        .keys()
        .filter(|a| is_valid_article(a))
        .cloned()
        .collect();
    articles.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    run_log::calc_info(
        log,
        format!(
            "[Калькулятор] Карточка WB: сборка {} артикулов…",
            articles.len()
        ),
    );

    let mut rows: Vec<Vec<String>> = Vec::with_capacity(articles.len() + 1);
    let wb_headers = catalog::wb_headers();
    rows.push(wb_headers.clone());

    let mut done = 0usize;
    for art in &articles {
        let a = maps.array.get(art);
        let f = maps.funnel.get(art);
        let st = maps.stocks.get(art);
        let p = maps.prices.get(art);
        let fb = maps.feedbacks.get(art);
        let pur = maps.purchases.get(art);

        let mut values: HashMap<String, String> = HashMap::new();
        // Массив: API-поле `photos` / photo1 / «Фото…» — в свод колонка «Фото» (без lookup-fallback с Массива).
        put(
            &mut values,
            "photo",
            photo_url(&cell(
                a,
                &[
                    "photo1",
                    "photos",
                    "Фото",
                    "Фото (до 30 ссылок в конце)",
                ],
            )),
        );
        put(&mut values, "cabinet", cell(a, &["Кабинет", "cabinet"]));
        put(
            &mut values,
            "name",
            cell(a, &["Наименование", "Название", "title"]),
        );
        put(&mut values, "brand", cell(a, &["Бренд", "brandName", "brand"]));
        put(
            &mut values,
            "vendorCode",
            cell(a, &["Артикул продавца", "vendorCode"]),
        );
        put(
            &mut values,
            "category",
            cell(a, &["Предмет", "Категория", "subjectName"]),
        );
        put(
            &mut values,
            "subjectId",
            cell(a, &["ID предмета", "subjectID", "subjectId"]),
        );
        put(
            &mut values,
            "dimLength",
            cell(a, &["Длина, см", "Длина", "dimensions.length"]),
        );
        put(
            &mut values,
            "dimWidth",
            cell(a, &["Ширина, см", "Ширина", "dimensions.width"]),
        );
        put(
            &mut values,
            "dimHeight",
            cell(a, &["Высота, см", "Высота", "dimensions.height"]),
        );
        put(
            &mut values,
            "dimWeight",
            cell(a, &["Вес, кг", "Вес", "dimensions.weightBrutto"]),
        );
        put(
            &mut values,
            "tnved",
            first_nonempty_line(&cell(a, &["ТН-ВЭД", "tnved"])),
        );
        put(
            &mut values,
            "docNumber",
            first_nonempty_line(&cell(
                a,
                &["Номер сертификата/декларации", "doc_number", "cert_number"],
            )),
        );
        put(
            &mut values,
            "docStart",
            first_nonempty_line(&cell(
                a,
                &[
                    "Дата регистрации сертификата/декларации",
                    "cert_reg_date",
                ],
            )),
        );
        put(
            &mut values,
            "docEnd",
            first_nonempty_line(&cell(
                a,
                &[
                    "Дата окончания действия сертификата/декларации",
                    "cert_expire_date",
                ],
            )),
        );
        put(
            &mut values,
            "docCheckStatus",
            first_nonempty_line(&cell(
                a,
                &["Статус проверки", "cert_check_status"],
            )),
        );

        let fbo = maps
            .funnel_stock_fbo
            .get(art)
            .cloned()
            .unwrap_or_else(|| cell(f, &["Остатки WB", "Остаток FBO"]));
        put(&mut values, "stockFbo", fbo);
        put(
            &mut values,
            "stockFbs",
            cell(st, &["Остаток", "Остаток FBS", "quantity"]),
        );
        put(
            &mut values,
            "clicks",
            cell(f, &["Переходы", "Клики", "selected.openCount"]),
        );
        put(
            &mut values,
            "clicksPrev",
            cell(f, &["Переходы (прошл.)", "Клики (прошл.)", "past.openCount"]),
        );
        put(
            &mut values,
            "cart",
            cell(f, &["В корзину", "Корзина", "selected.cartCount"]),
        );
        put(
            &mut values,
            "cartPrev",
            cell(f, &["В корзину (прошл.)", "Корзина (прошл.)", "past.cartCount"]),
        );
        put(
            &mut values,
            "wishlist",
            cell(f, &["В отложенное", "В желаемое", "selected.addToWishlist"]),
        );
        put(&mut values, "orders", cell(f, &["Заказы", "selected.orderCount"]));
        put(
            &mut values,
            "ordersPrev",
            cell(f, &["Заказы (прошл.)", "past.orderCount"]),
        );
        put(
            &mut values,
            "cancels",
            cell(f, &["Отмены", "Отмены, шт", "selected.cancelCount"]),
        );
        put(
            &mut values,
            "cancelsPrev",
            cell(f, &["Отмены (прошл.)", "past.cancelCount"]),
        );
        put(&mut values, "sales", cell(f, &["Выкупы", "selected.buyoutCount"]));
        put(
            &mut values,
            "salesPrev",
            cell(f, &["Выкупы (прошл.)", "past.buyoutCount"]),
        );
        put(
            &mut values,
            "rating",
            cell(
                fb,
                &[
                    "Рейтинг по отзывам (текущий)",
                    "Оценка пользователей",
                    "Рейтинг",
                ],
            ),
        );
        put(
            &mut values,
            "feedbacks",
            cell(
                fb,
                &["Все отзывы за период (текущий)", "Отзывы", "feedbacks"],
            ),
        );
        put(&mut values, "price", cell(p, &["Цена", "price"]));
        put(
            &mut values,
            "discount",
            cell(p, &["Скидка, %", "Скидка", "discount"]),
        );
        put(
            &mut values,
            "priceAfterDiscount",
            cell(
                p,
                &["Цена со скидкой (WB)", "Цена со скидкой", "discountedPrice"],
            ),
        );
        // Закуп из «Закупы» (авто) или ниже из manual bindings.
        put(&mut values, "purchase", cell(pur, &["Закуп"]));
        apply_manual_to_values(&mut values, art, &manual_maps);

        apply_commission_from_map(&mut values, &maps.commissions);
        let len = sheet_util::try_parse_number(
            values.get("dimLength").map(|s| s.as_str()).unwrap_or(""),
        )
        .unwrap_or(0.0);
        let wid = sheet_util::try_parse_number(
            values.get("dimWidth").map(|s| s.as_str()).unwrap_or(""),
        )
        .unwrap_or(0.0);
        let hei = sheet_util::try_parse_number(
            values.get("dimHeight").map(|s| s.as_str()).unwrap_or(""),
        )
        .unwrap_or(0.0);
        let weight = sheet_util::try_parse_number(
            values.get("dimWeight").map(|s| s.as_str()).unwrap_or(""),
        )
        .unwrap_or(0.0);
        let is_sgt = profile.wb_kgt.is_sgt(len, wid, hei, weight);
        apply_logistics_from_box(&mut values, maps.box_rows.as_ref(), is_sgt);

        if let Some(ad) = maps.ad_by_nm.get(art) {
            values.insert("hasAds".into(), if ad.has_ads { "да" } else { "нет" }.into());
            values.insert("adExpenses".into(), sheet_util::format_ru_num(ad.sum));
        } else {
            values.insert("hasAds".into(), "нет".into());
            values.insert("adExpenses".into(), "0".into());
        }

        apply_net_orders(&mut values);
        fill_order_cost(&mut values);

        if values.is_empty() {
            continue;
        }
        let mut by_header: HashMap<String, String> = HashMap::new();
        by_header.insert(t("calc.headers.wb.article"), art.clone());
        for &(key, header_key) in VALUE_TO_HEADER {
            if let Some(v) = values.get(key) {
                by_header.insert(t(header_key), v.clone());
            }
        }
        let mut row = Vec::with_capacity(wb_headers.len());
        for h in &wb_headers {
            row.push(by_header.get(h).cloned().unwrap_or_default());
        }
        rows.push(row);
        done += 1;
        if done % 500 == 0 {
            run_log::calc_info(
                log,
                format!(
                    "[Калькулятор] Карточка WB: прогресс {done}/{}…",
                    articles.len()
                ),
            );
        }
    }

    run_log::calc_info(log, "[Калькулятор] Карточка WB: запись в Google Sheets…");
    gs.clear_and_write(&sid, &table.sheet_name_tr(), rows, true).await?;
    let now = sheet_dates::format_datetime(Local::now().naive_local());
    profile
        .last_updated
        .insert(WB_SUMMARY_KEY.to_string(), now);
    run_log::calc_info(log, format!("[Калькулятор] «Карточка WB»: {done} строк."));
    crate::calculator::article_index::rebuild_after_wb_keys(profile, gs, articles, log).await;
    Ok(done)
}

async fn diagnose_empty_array(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: &AppLog,
) {
    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Array"))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        run_log::calc_info(log, "[Калькулятор] Карточка WB: нет ссылки на Массив.");
        return;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        run_log::calc_info(
            log,
            "[Калькулятор] Карточка WB: не удалось извлечь spreadsheetId Массива.",
        );
        return;
    };
    let sheet = catalog::find_table("Array")
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| t("calc.tables.Array.sheetName"));
    match get_sheet_rows_res(gs, cache, &sid, &sheet).await {
        Ok(rows) => {
            if rows.len() < 2 {
                run_log::calc_info(
                    log,
                    format!("[Калькулятор] Карточка WB: лист «{sheet}» пуст (<2 строк)."),
                );
                return;
            }
            let header = header_row(&rows);
            let art_idx = find_header_index(&header, "Артикул WB", ARTICLE_ALIASES);
            if art_idx < 0 {
                run_log::calc_info(
                    log,
                    format!(
                        "[Калькулятор] Карточка WB: на «{sheet}» нет колонки артикула (заголовки: {}).",
                        header.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                    ),
                );
                return;
            }
            let mut n = 0usize;
            for row in rows.iter().skip(1) {
                if is_valid_article(&cell_at(row, art_idx)) {
                    n += 1;
                }
            }
            run_log::calc_info(
                log,
                format!(
                    "[Калькулятор] Карточка WB: на «{sheet}» валидных артикулов={n} (строк данных={}).",
                    rows.len().saturating_sub(1)
                ),
            );
        }
        Err(e) => {
            run_log::calc_info(
                log,
                format!("[Калькулятор] Карточка WB: ошибка чтения Массива «{sheet}»: {e}"),
            );
        }
    }
}
