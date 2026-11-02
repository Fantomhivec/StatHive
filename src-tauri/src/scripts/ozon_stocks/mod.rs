//! OZON: Остатки — POST /v3/product/info/list.
//! Паритет `OZ_StocksScript`.

use serde_json::Value;
use crate::core::rate_profiles::ozon::stocks as rate;
use crate::core::script::{Script, ScriptContext, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_list_run::{self, ListRunParams};
use std::future::Future;
use std::pin::Pin;

pub struct OzonStocksScript;

impl Script for OzonStocksScript {
    fn key(&self) -> &'static str {
        "ozon_stocks"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_stocks")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move {
            ozon_list_run::run_multi_rows_per_item(
                ctx,
                ListRunParams {
                    script_label: "OZON: Остатки",
                    scope: rate::SCOPE,
                    profile: rate::profile(),
                    empty_ids_message: "Нет идентификаторов в источнике",
                    rows_label: "остатков",
                },
                common::resolve_fields(ctx, default_fields()),
                append_item_rows,
            )
            .await
        })
    }
}

// --- fields (merged) ---
use crate::scripts::ozon_util::json_str;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_stocks.cabinet"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_stocks.product_id"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_stocks.offer_id"), true),
        ScriptField::new("has_stock", crate::core::i18n::t("fields.ozon_stocks.has_stock"), true),
        ScriptField::new("present", crate::core::i18n::t("fields.ozon_stocks.present"), true),
        ScriptField::new("reserved", crate::core::i18n::t("fields.ozon_stocks.reserved"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_stocks.sku"), true),
        ScriptField::new("source", crate::core::i18n::t("fields.ozon_stocks.source"), true),
    ]
}

fn cell(map: &HashMap<&str, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_rows(
    cabinet: &str,
    item: &Value,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut offer_id = json_str(item.get("offer_id"));
    let mut product_id = json_str(item.get("id").or_else(|| item.get("product_id")));
    if let Some(src) = source_id {
        if id_mode == "offer_id" && offer_id.is_empty() {
            offer_id = src.to_string();
        } else if id_mode == "product_id" && product_id.is_empty() {
            product_id = src.to_string();
        }
    }

    let has_stock = item
        .pointer("/stocks/has_stock")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let Some(stocks) = item.pointer("/stocks/stocks").and_then(|a| a.as_array()) else {
        return Vec::new();
    };
    if stocks.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    for stock in stocks {
        let mut map = HashMap::new();
        map.insert("cabinet", cabinet.to_string());
        map.insert("offer_id", offer_id.clone());
        map.insert("product_id", product_id.clone());
        map.insert(
            "has_stock",
            if has_stock {
                "TRUE".into()
            } else {
                "FALSE".into()
            },
        );
        map.insert("present", json_str(stock.get("present")));
        map.insert("reserved", json_str(stock.get("reserved")));
        map.insert("sku", json_str(stock.get("sku")));
        map.insert("source", json_str(stock.get("source")));
        rows.push(selected.iter().map(|f| cell(&map, &f.name)).collect());
    }
    rows
}

pub fn append_item_rows(
    all: &mut Vec<Vec<String>>,
    header_done: &mut bool,
    cabinet: &str,
    item: &Value,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
) {
    let chunk = build_rows(cabinet, item, fields, id_mode, source_id);
    if chunk.is_empty() {
        return;
    }
    common::ensure_header_row(all, header_done, fields);
    all.extend(chunk);
}
