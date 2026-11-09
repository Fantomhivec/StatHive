// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    /// dd.MM.yyyy when script was last reconciled with official API docs; empty if unknown.
    pub api_updated: String,
}

/// WB vs OZON: ключи `ozon_*` только для Ozon; всё остальное — WB. Площадки не пересекаются.
pub fn script_belongs_to_marketplace(script_key: &str, marketplace: &str) -> bool {
    let market = marketplace.trim().to_ascii_lowercase();
    let is_ozon = script_key.trim().starts_with("ozon_");
    if market == "ozon" {
        is_ozon
    } else {
        !is_ozon
    }
}

/// Первый ключ из групп площадки, который проходит ACL; иначе запасной дефолт площадки.
pub fn default_script_key(
    registry: &ScriptRegistry,
    marketplace: &str,
    can_run: &dyn Fn(&str, bool) -> bool,
) -> String {
    let market = marketplace.trim().to_ascii_lowercase();
    for g in list_grouped(registry, &market) {
        for s in g.scripts {
            let member_ok = registry.is_member_ok(&s.key);
            if can_run(&s.key, member_ok) {
                return s.key;
            }
        }
    }
    if market == "ozon" {
        "ozon_product_list".into()
    } else {
        "cards_list".into()
    }
}

/// Группы `ScriptGroupsConfiguration` — только зарегистрированные скрипты.
pub fn list_grouped(registry: &ScriptRegistry, marketplace: &str) -> Vec<ScriptGroupDto> {
    let market = marketplace.trim().to_ascii_lowercase();
    let defs: &[(&str, &[(&str, &str, &str)])] = if market == "ozon" {
        &[
            (
                "groups.ozon.products",
                &[
                    ("ozon_product_list", "POST /v3/product/list", ""),
                    ("ozon_product_info", "POST /v3/product/info/list", ""),
                    ("ozon_product_prices", "POST /v5/product/info/prices", ""),
                    ("ozon_commissions", "POST /v3/product/info/list", ""),
                    ("ozon_subscription", "POST /v1/product/info/subscription", ""),
                    ("ozon_product_attributes", "POST /v4/product/info/attributes", ""),
                ],
            ),
            (
                "groups.ozon.reports",
                &[("ozon_product_report", "POST /v1/report/products/create", "")],
            ),
            (
                "groups.ozon.stocks",
                &[
                    ("ozon_stocks", "POST /v3/product/info/list", ""),
                    ("ozon_fbs_stocks", "POST /v4/product/info/stocks", ""),
                ],
            ),
            (
                "groups.ozon.analytics",
                &[("ozon_average_delivery_time", "POST /v1/analytics/average-delivery-time/summary", "")],
            ),
            (
                "groups.ozon.orders",
                &[
                    ("ozon_fbo_orders", "POST /v3/posting/fbo/list", ""),
                    ("ozon_fbs_orders", "POST /v4/posting/fbs/list", ""),
                ],
            ),
            (
                "groups.ozon.coinvest",
                &[("ozon_coinvest", "GET /v1/actions + POST /v2/actions/candidates", "")],
            ),
        ]
    } else {
        &[
            (
                "groups.wb.productsEmoji",
                &[
                    ("cards_list", "POST /content/v2/get/cards/list", "06.11.2026"),
                    ("cards_trash", "POST /content/v2/get/cards/trash", "06.11.2026"),
                    ("cards_move_to_trash", "POST /content/v2/cards/delete/trash", "06.11.2026"),
                    ("cards_update_dimensions", "POST /content/v2/cards/update", "06.11.2026"),
                    ("cards_error_list", "POST /content/v2/cards/error/list", "06.11.2026"),
                    ("cards_move_nm", "POST /content/v2/cards/moveNm", "06.11.2026"),
                    ("content_media_save", "POST /content/v3/media/save", "06.11.2026"),
                    ("categories_parent", "GET /content/v2/object/parent/all", "06.11.2026"),
                    ("subjects_list", "GET /content/v2/object/all", "06.11.2026"),
                    ("wb_tnved_codes", "GET /content/v2/directory/tnved", "09.11.2026"),
                    ("wb_object_charcs", "GET /content/v2/object/charcs/{subjectId}", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.prices",
                &[
                    ("prices_export", "GET /api/v2/list/goods/filter", "09.11.2026"),
                    ("prices_by_nm", "POST /api/v2/list/goods/filter", "09.11.2026"),
                    ("prices_upload", "POST /api/v2/upload/task", "09.11.2026"),
                    ("prices_set", "POST /api/v2/upload/task", "09.11.2026"),
                    ("prices_history_task", "GET /api/v2/history/goods/task", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.finance",
                &[
                    ("report_realization_v5", "POST /api/finance/v1/sales-reports/detailed", "09.11.2026"),
                    ("sales_reports_list", "POST /api/finance/v1/sales-reports/list", "09.11.2026"),
                    ("paid_storage_create_task", "GET /api/v1/paid_storage", "09.11.2026"),
                    ("paid_storage_status", "GET /api/v1/paid_storage/tasks/{id}/status", "09.11.2026"),
                    ("paid_storage_download", "GET /api/v1/paid_storage/tasks/{id}/download", "09.11.2026"),
                    ("paid_storage_auto", "GET paid_storage create+status+download", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.deductions",
                &[
                    ("warehouse_self_purchases", "GET /api/v1/analytics/antifraud-details", "09.11.2026"),
                    ("goods_labeling", "GET /api/v1/analytics/goods-labeling", "09.11.2026"),
                    ("analytics_deductions", "GET /api/analytics/v1/deductions", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.tariffs",
                &[
                    ("tariffs_commission", "GET /api/v1/tariffs/commission", "09.11.2026"),
                    ("tariffs_box", "GET /api/v1/tariffs/box", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.advertising",
                &[
                    ("adv_campaigns_list", "GET adv/v1/promotion/count", "09.11.2026"),
                    ("wb_adv_campaigns_stats_new", "GET /api/advert/v2/adverts", "09.11.2026"),
                    ("wb_adv_fullstats", "GET /adv/v3/fullstats", "09.11.2026"),
                    ("wb_adv_norm_query_stats", "POST /adv/v1/normquery/stats", "09.11.2026"),
                    ("wb_adv_norm_query_bids", "POST /adv/v0/normquery/get-bids", "09.11.2026"),
                    ("wb_adv_search_texts", "POST /api/v2/search-report/product/search-texts", "09.11.2026"),
                    ("wb_adv_search_report_main", "POST /api/v2/search-report/report", "09.11.2026"),
                    ("wb_adv_upd", "GET /adv/v1/upd", "09.11.2026"),
                    ("promotions", "GET /api/v1/calendar/promotions", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.funnel",
                &[
                    ("sales_funnel_products", "POST /api/analytics/v3/sales-funnel/products", "09.11.2026"),
                    ("sales_funnel_products_history", "POST /api/analytics/v3/sales-funnel/products/history", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.orders",
                &[
                    ("order_feed", "POST /api/analytics/v1/order-feed", "09.11.2026"),
                    (
                        "wb_supplier_orders",
                        "GET /api/v1/supplier/orders",
                        "09.11.2026"
                    ),
                    ("orders_fbs", "GET /api/v3/orders", "09.11.2026"),
                    ("orders_fbs_status", "POST /api/v3/orders/status", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.nm_report",
                &[
                    ("nm_report_downloads", "POST /api/v2/nm-report/downloads", "09.11.2026"),
                    ("nm_report_downloads_list", "GET /api/v2/nm-report/downloads", "09.11.2026"),
                    ("nm_report_download_file", "GET /api/v2/nm-report/downloads/file/{id}", "09.11.2026"),
                    ("nm_report_downloads_auto", "nm-report create+list+file", "09.11.2026"),
                    ("nm_report_stock_history_auto", "STOCK_HISTORY_DAILY_CSV auto", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.stocks",
                &[
                    ("wb_stocks", "POST /api/analytics/v1/stocks-report/seller-warehouses", "09.11.2026"),
                    ("wb_stock_fbo_report", "POST /api/analytics/v1/stocks-report/wb-warehouses", "09.11.2026"),
                    ("warehouse_remains_create_task", "GET /api/v1/warehouse_remains", "09.11.2026"),
                    ("warehouse_remains_status", "GET /api/v1/warehouse_remains/tasks/{id}/status", "09.11.2026"),
                    ("warehouse_remains_download", "GET /api/v1/warehouse_remains/tasks/{id}/download", "09.11.2026"),
                    ("warehouse_remains_auto", "create→status→download warehouse_remains", "09.11.2026"),
                    ("warehouses_list", "GET /api/v3/warehouses", "09.11.2026"),
                    ("wb_offices", "GET /api/v3/offices", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.supplies",
                &[
                    ("supplies", "GET /api/v3/supplies", "09.11.2026"),
                    ("supplies_fbo", "POST /api/v1/supplies", "09.11.2026"),
                    ("supply_info", "GET /api/v3/supplies/{id}", "09.11.2026"),
                    ("supplies_goods", "GET /api/v1/supplies/{id}/goods", "09.11.2026"),
                    ("supplies_warehouses", "GET /api/v1/warehouses", "09.11.2026"),
                    ("acceptance_options", "POST /api/v1/acceptance/options", "09.11.2026"),
                    ("transit_tariffs", "GET /api/v1/transit-tariffs", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.feedbacks",
                &[
                    ("item_rating", "POST /api/analytics/v2/item-rating", "09.11.2026"),
                    ("feedbacks_archive", "GET /api/v1/feedbacks/archive", "09.11.2026"),
                    ("feedbacks_count", "GET /api/v1/feedbacks/count", "09.11.2026"),
                    ("claims_list", "GET /api/v1/claims", "09.11.2026"),
                ],
            ),
            (
                "groups.wb.utils",
                &[
                    ("copy_data", "Sheets copy range", "09.11.2026"),
                    ("symbols_check", "Sheets textFormatRuns EN/RU", "09.11.2026"),
                    ("cells_count", "Sheets grid cell count", "09.11.2026"),
                ],
            ),
        ]
    };

    let mut out = Vec::new();
    for (name_key, scripts) in defs {
        let mut opts = Vec::new();
        for (key, api, updated) in *scripts {
            if let Some(s) = registry.get(key) {
                opts.push(ScriptOptionDto {
                    key: key.to_string(),
                    title: s.title(),
                    api_method: api.to_string(),
                    api_updated: updated.to_string(),
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
        .flat_map(|(_, scripts)| scripts.iter().map(|(k, _, _)| *k))
        .collect();
    let mut other = Vec::new();
    for (key, title) in registry.list() {
        if script_belongs_to_marketplace(&key, &market) && !known.contains(key.as_str()) {
            other.push(ScriptOptionDto {
                key,
                title,
                api_method: String::new(),
                api_updated: String::new(),
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
