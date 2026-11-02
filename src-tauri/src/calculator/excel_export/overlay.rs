//! Ручные привязки поверх свода (паритет `OverlayManualBindingsAsync`).
//!
//! Основной путь: свод уже содержит поля (в т.ч. закуп из manual при сборке).
//! Overlay — только для таблиц в режиме manual (свежие правки без пересборки свода).

use super::data::DataTable;
use crate::calculator::catalog;
use crate::core::i18n::t;
use crate::calculator::manual_bindings::{self, map_get_public};
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::sheet_util::SheetCache;
use crate::core::google_sheets::GoogleSheetsClient;

pub async fn overlay_manual_bindings(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    table: &mut DataTable,
    is_ozon: bool,
    warnings: &mut Vec<String>,
) {
    let mp = if is_ozon { "ozon" } else { "wb" };
    let art_header = if is_ozon {
        t("calc.headers.oz.sku")
    } else {
        t("calc.headers.wb.article")
    };
    let maps = manual_bindings::load_manual_metric_maps(
        profile,
        mp,
        gs,
        cache,
        None,
        Some(warnings),
    )
    .await;

    for metric in catalog::metrics_for_marketplace(mp) {
        let Some(art_map) = maps.get(metric.key) else {
            continue;
        };
        let out_header = if metric.default_column_header.trim().is_empty() {
            metric.label_tr()
        } else {
            metric.default_column_header_tr()
        };
        table.ensure_header(&out_header);

        for row in &mut table.rows {
            let Some(art) = row.get(&art_header).map(|s| s.trim().to_string()) else {
                continue;
            };
            if art.is_empty() {
                continue;
            }
            if let Some(val) = map_get_public(art_map, &art) {
                row.insert(out_header.to_string(), val.clone());
            }
        }
    }
}
