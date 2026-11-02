//! Загрузка листов в карты артикул→колонки + BuildAdExpensesByNm (паритет).

use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::{
    SUBJECT_ALIASES,
    WB_SOURCE_ARTICLE_HEADERS, SheetCache, SheetRows,
};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use std::collections::HashMap;
use super::sheet_map::{build_ad_expenses_by_nm, load_sheet_map, load_sum_column_map, load_table_rows};


pub type RowMap = HashMap<String, HashMap<String, String>>;

pub struct AdAgg {
    pub has_ads: bool,
    pub sum: f64,
}

pub struct WbMaps {
    pub array: RowMap,
    pub funnel: RowMap,
    pub stocks: RowMap,
    pub prices: RowMap,
    pub feedbacks: RowMap,
    pub purchases: RowMap,
    pub commissions: RowMap,
    pub funnel_stock_fbo: HashMap<String, String>,
    pub box_rows: Option<SheetRows>,
    pub ad_by_nm: HashMap<String, AdAgg>,
}

pub(crate) fn merge_sheet_cache(into: &mut SheetCache, from: SheetCache) {
    for (k, v) in from {
        into.entry(k).or_insert(v);
    }
}

pub async fn load_wb_maps(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    log: &AppLog,
) -> WbMaps {
    // nmId-заголовки для всех источников свода (в т.ч. Закупы → колонка «Закуп»).
    let art = WB_SOURCE_ARTICLE_HEADERS;
    run_log::calc_info(log, "[Калькулятор] Карточка WB: загрузка исходных листов…");

    // Параллельно: отдельный SheetCache на задачу, затем merge (без гонки на одном HashMap).
    let (
        (array, c_array),
        (funnel, c_funnel),
        (stocks, c_stocks),
        (prices, c_prices),
        (feedbacks, c_feedbacks),
        (purchases, c_purchases),
        (commissions, c_commissions),
        (box_rows, c_box),
    ) = tokio::join!(
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Array", gs, &mut c, art, false, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Funnel", gs, &mut c, art, false, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Stocks", gs, &mut c, art, true, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Prices", gs, &mut c, art, false, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Feedbacks", gs, &mut c, art, false, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(profile, "Purchases", gs, &mut c, art, false, None, log).await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let m = load_sheet_map(
                profile,
                "Commissions",
                gs,
                &mut c,
                SUBJECT_ALIASES,
                false,
                None,
                log,
            )
            .await;
            (m, c)
        },
        async {
            let mut c = SheetCache::new();
            let rows = load_table_rows(profile, "BoxTariffs", gs, &mut c, None, log).await;
            (rows, c)
        },
    );

    for c in [
        c_array,
        c_funnel,
        c_stocks,
        c_prices,
        c_feedbacks,
        c_purchases,
        c_commissions,
        c_box,
    ] {
        merge_sheet_cache(cache, c);
    }

    // Funnel уже в cache — sum без повторного GET. Реклама отдельно (тот же &mut cache).
    let funnel_stock_fbo = load_sum_column_map(
        profile,
        "Funnel",
        gs,
        cache,
        WB_SOURCE_ARTICLE_HEADERS,
        &["Остатки WB", "Остаток FBO"],
        log,
    )
    .await;
    let ad_by_nm = build_ad_expenses_by_nm(profile, gs, cache, log).await;

    run_log::calc_info(
        log,
        format!(
            "[Калькулятор] Карточка WB: карты готовы (Массив={}, Воронка={}, Цены={}).",
            array.len(),
            funnel.len(),
            prices.len()
        ),
    );

    WbMaps {
        array,
        funnel,
        stocks,
        prices,
        feedbacks,
        purchases,
        commissions,
        funnel_stock_fbo,
        box_rows,
        ad_by_nm,
    }
}

