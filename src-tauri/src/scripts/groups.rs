use crate::core::i18n::t;
use crate::core::script::ScriptRegistry;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScriptGroupDto {
    pub name: String,
    pub scripts: Vec<ScriptOptionDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptOptionDto {
    pub key: String,
    pub title: String,
    pub api_method: String,
}

/// Группы `ScriptGroupsConfiguration` — только зарегистрированные скрипты.
pub fn list_grouped(registry: &ScriptRegistry, marketplace: &str) -> Vec<ScriptGroupDto> {
    let market = marketplace.trim().to_ascii_lowercase();
    let defs: &[(&str, &[(&str, &str)])] = if market == "ozon" {
        &[
            (
                "groups.ozon.products",
                &[
                    ("ozon_product_list", "POST /v3/product/list"),
                    ("ozon_product_info", "POST /v3/product/info/list"),
                    ("ozon_product_prices", "POST /v5/product/info/prices"),
                    ("ozon_commissions", "POST /v3/product/info/list"),
                    ("ozon_subscription", "POST /v1/product/info/subscription"),
                    ("ozon_product_attributes", "POST /v4/product/info/attributes"),
                ],
            ),
            (
                "groups.ozon.reports",
                &[("ozon_product_report", "POST /v1/report/products/create")],
            ),
            (
                "groups.ozon.stocks",
                &[
                    ("ozon_stocks", "POST /v3/product/info/list"),
                    ("ozon_fbs_stocks", "POST /v4/product/info/stocks"),
                ],
            ),
            (
                "groups.ozon.analytics",
                &[(
                    "ozon_average_delivery_time",
                    "POST /v1/analytics/average-delivery-time/summary",
                )],
            ),
            (
                "groups.ozon.orders",
                &[
                    ("ozon_fbo_orders", "POST /v3/posting/fbo/list"),
                    ("ozon_fbs_orders", "POST /v4/posting/fbs/list"),
                ],
            ),
            (
                "groups.ozon.coinvest",
                &[(
                    "ozon_coinvest",
                    "GET /v1/actions + POST /v2/actions/candidates",
                )],
            ),
        ]
    } else {
        &[
            (
                "groups.wb.productsEmoji",
                &[
                    ("cards_list", "POST /content/v2/get/cards/list"),
                    ("cards_trash", "POST /content/v2/get/cards/trash"),
                    ("cards_move_to_trash", "POST /content/v2/cards/delete/trash"),
                    ("cards_update_dimensions", "POST /content/v2/cards/update"),
                    ("cards_error_list", "POST /content/v2/cards/error/list"),
                    ("cards_move_nm", "POST /content/v2/cards/moveNm"),
                    ("content_media_save", "POST /content/v3/media/save"),
                    ("categories_parent", "GET /content/v2/object/parent/all"),
                    ("subjects_list", "GET /content/v2/object/all"),
                    ("wb_tnved_codes", "GET /content/v2/directory/tnved"),
                    (
                        "wb_object_charcs",
                        "GET /content/v2/object/charcs/{subjectId}",
                    ),
                ],
            ),
            (
                "groups.wb.prices",
                &[
                    ("prices_export", "GET /api/v2/list/goods/filter"),
                    ("prices_by_nm", "POST /api/v2/list/goods/filter"),
                    ("prices_upload", "POST /api/v2/upload/task"),
                    ("prices_set", "POST /api/v2/upload/task"),
                    ("prices_history_task", "GET /api/v2/history/goods/task"),
                ],
            ),
            (
                "groups.wb.finance",
                &[
                    (
                        "report_realization_v5",
                        "POST /api/finance/v1/sales-reports/detailed",
                    ),
                    (
                        "sales_reports_list",
                        "POST /api/finance/v1/sales-reports/list",
                    ),
                    (
                        "paid_storage_create_task",
                        "GET /api/v1/paid_storage",
                    ),
                    (
                        "paid_storage_status",
                        "GET /api/v1/paid_storage/tasks/{id}/status",
                    ),
                    (
                        "paid_storage_download",
                        "GET /api/v1/paid_storage/tasks/{id}/download",
                    ),
                    (
                        "paid_storage_auto",
                        "GET paid_storage create+status+download",
                    ),
                ],
            ),
            (
                "groups.wb.deductions",
                &[
                    (
                        "warehouse_self_purchases",
                        "GET /api/v1/analytics/antifraud-details",
                    ),
                    (
                        "goods_labeling",
                        "GET /api/v1/analytics/goods-labeling",
                    ),
                    (
                        "analytics_deductions",
                        "GET /api/analytics/v1/deductions",
                    ),
                ],
            ),
            (
                "groups.wb.tariffs",
                &[
                    ("tariffs_commission", "GET /api/v1/tariffs/commission"),
                    ("tariffs_box", "GET /api/v1/tariffs/box"),
                ],
            ),
            (
                "groups.wb.advertising",
                &[
                    ("adv_campaigns_list", "GET adv/v1/promotion/count"),
                    (
                        "wb_adv_campaigns_stats_new",
                        "GET /api/advert/v2/adverts",
                    ),
                    ("wb_adv_fullstats", "GET /adv/v3/fullstats"),
                    (
                        "wb_adv_norm_query_stats",
                        "POST /adv/v0/normquery/stats",
                    ),
                    (
                        "wb_adv_norm_query_bids",
                        "POST /adv/v0/normquery/get-bids",
                    ),
                    (
                        "wb_adv_search_texts",
                        "POST /api/v2/search-report/product/search-texts",
                    ),
                    (
                        "wb_adv_search_report_main",
                        "POST /api/v2/search-report/report",
                    ),
                    ("wb_adv_upd", "GET /adv/v1/upd"),
                    (
                        "promotions",
                        "GET /api/v1/calendar/promotions",
                    ),
                ],
            ),
            (
                "groups.wb.funnel",
                &[
                    (
                        "sales_funnel_products",
                        "POST /api/analytics/v3/sales-funnel/products",
                    ),
                    (
                        "sales_funnel_products_history",
                        "POST /api/analytics/v3/sales-funnel/products/history",
                    ),
                ],
            ),
            (
                "groups.wb.orders",
                &[
                    (
                        "order_feed",
                        "POST /api/analytics/v1/order-feed",
                    ),
                    ("orders_fbs", "GET /api/v3/orders"),
                    ("orders_fbs_status", "POST /api/v3/orders/status"),
                ],
            ),
            (
                "groups.wb.nm_report",
                &[
                    (
                        "nm_report_downloads",
                        "POST /api/v2/nm-report/downloads",
                    ),
                    (
                        "nm_report_downloads_list",
                        "GET /api/v2/nm-report/downloads",
                    ),
                    (
                        "nm_report_download_file",
                        "GET /api/v2/nm-report/downloads/file/{id}",
                    ),
                    (
                        "nm_report_downloads_auto",
                        "nm-report create+list+file",
                    ),
                    (
                        "nm_report_stock_history_auto",
                        "STOCK_HISTORY_DAILY_CSV auto",
                    ),
                ],
            ),
            (
                "groups.wb.stocks",
                &[
                    (
                        "wb_stocks",
                        "POST /api/analytics/v1/stocks-report/seller-warehouses",
                    ),
                    (
                        "wb_stock_fbo_report",
                        "POST /api/analytics/v1/stocks-report/wb-warehouses",
                    ),
                    (
                        "warehouse_remains_create_task",
                        "GET /api/v1/warehouse_remains",
                    ),
                    (
                        "warehouse_remains_status",
                        "GET /api/v1/warehouse_remains/tasks/{id}/status",
                    ),
                    (
                        "warehouse_remains_download",
                        "GET /api/v1/warehouse_remains/tasks/{id}/download",
                    ),
                    (
                        "warehouse_remains_auto",
                        "create→status→download warehouse_remains",
                    ),
                    ("warehouses_list", "GET /api/v3/warehouses"),
                    ("wb_offices", "GET /api/v3/offices"),
                ],
            ),
            (
                "groups.wb.supplies",
                &[
                    ("supplies", "GET /api/v3/supplies"),
                    ("supplies_fbo", "POST /api/v1/supplies"),
                    ("supply_info", "GET /api/v3/supplies/{id}"),
                    ("supplies_goods", "GET /api/v1/supplies/{id}/goods"),
                    ("supplies_warehouses", "GET /api/v1/warehouses"),
                    (
                        "acceptance_options",
                        "POST /api/v1/acceptance/options",
                    ),
                    ("transit_tariffs", "GET /api/v1/transit-tariffs"),
                ],
            ),
            (
                "groups.wb.feedbacks",
                &[
                    ("item_rating", "POST /api/analytics/v2/item-rating"),
                    (
                        "feedbacks_archive",
                        "GET /api/v1/feedbacks/archive",
                    ),
                    ("feedbacks_count", "GET /api/v1/feedbacks/count"),
                    ("claims_list", "GET /api/v1/claims"),
                ],
            ),
            (
                "groups.wb.utils",
                &[
                    ("copy_data", "Sheets copy range"),
                    ("symbols_check", "Sheets textFormatRuns EN/RU"),
                    ("cells_count", "Sheets grid cell count"),
                ],
            ),
        ]
    };

    let mut out = Vec::new();
    for (name_key, scripts) in defs {
        let mut opts = Vec::new();
        for (key, api) in *scripts {
            if let Some(s) = registry.get(key) {
                opts.push(ScriptOptionDto {
                    key: key.to_string(),
                    title: s.title(),
                    api_method: api.to_string(),
                });
            }
        }
        if !opts.is_empty() {
            out.push(ScriptGroupDto {
                name: t(name_key),
                scripts: opts,
            });
        }
    }

    // Скрипты в registry без группы — «Прочее» (только своей площадки).
    let known: std::collections::HashSet<&str> = defs
        .iter()
        .flat_map(|(_, scripts)| scripts.iter().map(|(k, _)| *k))
        .collect();
    let mut other = Vec::new();
    for (key, title) in registry.list() {
        let is_ozon_script = key.starts_with("ozon_");
        let belongs = if market == "ozon" {
            is_ozon_script
        } else {
            !is_ozon_script
        };
        if belongs && !known.contains(key.as_str()) {
            other.push(ScriptOptionDto {
                key,
                title,
                api_method: String::new(),
            });
        }
    }
    if !other.is_empty() {
        let other_key = if market == "ozon" {
            "groups.ozon.other"
        } else {
            "groups.wb.other"
        };
        out.push(ScriptGroupDto {
            name: t(other_key),
            scripts: other,
        });
    }
    out
}
