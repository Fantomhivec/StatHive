//! Поля Ozon product prices (паритет OZ_ProductPricesScript).

use crate::core::script_field::ScriptField;
use crate::scripts::ozon_util::{format_number_cell, format_ozon_dt, json_str, yes_no};
use serde_json::Value;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_product_prices.cabinet"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_product_prices.sku"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_product_prices.product_id"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_product_prices.offer_id"), true),
        ScriptField::new("acquiring", crate::core::i18n::t("fields.ozon_product_prices.acquiring"), true),
        ScriptField::new("volume_weight", crate::core::i18n::t("fields.ozon_product_prices.volume_weight"), true),
        ScriptField::new("price", crate::core::i18n::t("fields.ozon_product_prices.price"), true),
        ScriptField::new("old_price", crate::core::i18n::t("fields.ozon_product_prices.old_price"), true),
        ScriptField::new("min_price", crate::core::i18n::t("fields.ozon_product_prices.min_price"), true),
        ScriptField::new("net_price", crate::core::i18n::t("fields.ozon_product_prices.net_price"), false),
        ScriptField::new("marketing_seller_price", crate::core::i18n::t("fields.ozon_product_prices.marketing_seller_price"), false),
        ScriptField::new("retail_price", crate::core::i18n::t("fields.ozon_product_prices.retail_price"), false),
        ScriptField::new("currency_code", crate::core::i18n::t("fields.ozon_product_prices.currency_code"), true),
        ScriptField::new("vat", crate::core::i18n::t("fields.ozon_product_prices.vat"), true),
        ScriptField::new("auto_action_enabled", crate::core::i18n::t("fields.ozon_product_prices.auto_action_enabled"), false),
        ScriptField::new(
            "auto_add_to_ozon_actions_list_enabled",
            "Автодобавление в акции Ozon",
            false,
        ),
        ScriptField::new("declared_price_amount", crate::core::i18n::t("fields.ozon_product_prices.declared_price_amount"), false),
        ScriptField::new("declared_price_currency", crate::core::i18n::t("fields.ozon_product_prices.declared_price_currency"), false),
        ScriptField::new("sales_percent_fbo", crate::core::i18n::t("fields.ozon_product_prices.sales_percent_fbo"), true),
        ScriptField::new("sales_percent_fbs", crate::core::i18n::t("fields.ozon_product_prices.sales_percent_fbs"), true),
        ScriptField::new("sales_percent_fbp", crate::core::i18n::t("fields.ozon_product_prices.sales_percent_fbp"), false),
        ScriptField::new("sales_percent_rfbs", crate::core::i18n::t("fields.ozon_product_prices.sales_percent_rfbs"), false),
        ScriptField::new("fbo_deliv_to_customer_amount", crate::core::i18n::t("fields.ozon_product_prices.fbo_deliv_to_customer_amount"), false),
        ScriptField::new("fbo_direct_flow_trans_min_amount", crate::core::i18n::t("fields.ozon_product_prices.fbo_direct_flow_trans_min_amount"), false),
        ScriptField::new("fbo_direct_flow_trans_max_amount", crate::core::i18n::t("fields.ozon_product_prices.fbo_direct_flow_trans_max_amount"), false),
        ScriptField::new("fbo_return_flow_amount", crate::core::i18n::t("fields.ozon_product_prices.fbo_return_flow_amount"), false),
        ScriptField::new("fbs_deliv_to_customer_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_deliv_to_customer_amount"), false),
        ScriptField::new("fbs_direct_flow_trans_min_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_direct_flow_trans_min_amount"), false),
        ScriptField::new("fbs_direct_flow_trans_max_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_direct_flow_trans_max_amount"), false),
        ScriptField::new("fbs_first_mile_min_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_first_mile_min_amount"), false),
        ScriptField::new("fbs_first_mile_max_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_first_mile_max_amount"), false),
        ScriptField::new("fbs_return_flow_amount", crate::core::i18n::t("fields.ozon_product_prices.fbs_return_flow_amount"), false),
        ScriptField::new("color_index", crate::core::i18n::t("fields.ozon_product_prices.color_index"), true),
        ScriptField::new("ozon_index_min_price", crate::core::i18n::t("fields.ozon_product_prices.ozon_index_min_price"), false),
        ScriptField::new("ozon_index_min_price_currency", crate::core::i18n::t("fields.ozon_product_prices.ozon_index_min_price_currency"), false),
        ScriptField::new("ozon_index_value", crate::core::i18n::t("fields.ozon_product_prices.ozon_index_value"), false),
        ScriptField::new("external_index_min_price", crate::core::i18n::t("fields.ozon_product_prices.external_index_min_price"), false),
        ScriptField::new("external_index_min_price_currency", crate::core::i18n::t("fields.ozon_product_prices.external_index_min_price_currency"), false),
        ScriptField::new("external_index_value", crate::core::i18n::t("fields.ozon_product_prices.external_index_value"), false),
        ScriptField::new(
            "self_marketplaces_index_min_price",
            "Индекс свои площадки: мин. цена",
            false,
        ),
        ScriptField::new(
            "self_marketplaces_index_min_price_currency",
            "Индекс свои площадки: валюта",
            false,
        ),
        ScriptField::new(
            "self_marketplaces_index_value",
            "Индекс свои площадки: значение",
            false,
        ),
        ScriptField::new("ozon_actions_exist", crate::core::i18n::t("fields.ozon_product_prices.ozon_actions_exist"), false),
        ScriptField::new("current_period_from", crate::core::i18n::t("fields.ozon_product_prices.current_period_from"), false),
        ScriptField::new("current_period_to", crate::core::i18n::t("fields.ozon_product_prices.current_period_to"), false),
        ScriptField::new("actions_titles", crate::core::i18n::t("fields.ozon_product_prices.actions_titles"), false),
        ScriptField::new("actions_values", crate::core::i18n::t("fields.ozon_product_prices.actions_values"), false),
        ScriptField::new("actions_date_from", crate::core::i18n::t("fields.ozon_product_prices.actions_date_from"), false),
        ScriptField::new("actions_date_to", crate::core::i18n::t("fields.ozon_product_prices.actions_date_to"), false),
    ]
}

fn cell(map: &HashMap<String, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

fn fill_index(map: &mut HashMap<String, String>, prefix: &str, index: Option<&Value>) {
    let Some(obj) = index.and_then(|v| v.as_object()) else {
        return;
    };
    let o = Value::Object(obj.clone());
    map.insert(
        format!("{prefix}_min_price"),
        format_number_cell(o.get("min_price")),
    );
    map.insert(
        format!("{prefix}_min_price_currency"),
        json_str(o.get("min_price_currency")),
    );
    map.insert(
        format!("{prefix}_value"),
        format_number_cell(o.get("price_index_value")),
    );
}

pub fn build_row(
    cabinet: &str,
    item: &Value,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
    resolved_sku: Option<&str>,
) -> Vec<String> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();

    let mut offer_id = json_str(item.get("offer_id"));
    let mut product_id = json_str(item.get("product_id"));
    let mut sku = resolved_sku
        .map(str::to_string)
        .unwrap_or_else(|| json_str(item.get("sku")));
    if let Some(src) = source_id {
        match id_mode {
            "offer_id" if offer_id.is_empty() => offer_id = src.to_string(),
            "product_id" if product_id.is_empty() => product_id = src.to_string(),
            "sku" if sku.is_empty() => sku = src.to_string(),
            _ => {}
        }
    }

    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    map.insert("sku".into(), sku);
    map.insert("product_id".into(), product_id);
    map.insert("offer_id".into(), offer_id);
    map.insert("acquiring".into(), format_number_cell(item.get("acquiring")));
    map.insert("volume_weight".into(), format_number_cell(item.get("volume_weight")));

    if let Some(price) = item.get("price") {
        map.insert("price".into(), format_number_cell(price.get("price")));
        map.insert("old_price".into(), format_number_cell(price.get("old_price")));
        map.insert("min_price".into(), format_number_cell(price.get("min_price")));
        map.insert("net_price".into(), format_number_cell(price.get("net_price")));
        map.insert(
            "marketing_seller_price".into(),
            format_number_cell(price.get("marketing_seller_price")),
        );
        map.insert("retail_price".into(), format_number_cell(price.get("retail_price")));
        map.insert("currency_code".into(), json_str(price.get("currency_code")));
        map.insert("vat".into(), format_number_cell(price.get("vat")));
        map.insert(
            "auto_action_enabled".into(),
            yes_no(price.get("auto_action_enabled").and_then(|x| x.as_bool())),
        );
        map.insert(
            "auto_add_to_ozon_actions_list_enabled".into(),
            yes_no(
                price
                    .get("auto_add_to_ozon_actions_list_enabled")
                    .and_then(|x| x.as_bool()),
            ),
        );
        if let Some(decl) = price.get("declared_price") {
            map.insert(
                "declared_price_amount".into(),
                format_number_cell(decl.get("amount")),
            );
            map.insert(
                "declared_price_currency".into(),
                json_str(decl.get("currency")),
            );
        }
    }

    if let Some(c) = item.get("commissions") {
        for key in [
            "sales_percent_fbo",
            "sales_percent_fbs",
            "sales_percent_fbp",
            "sales_percent_rfbs",
            "fbo_deliv_to_customer_amount",
            "fbo_direct_flow_trans_min_amount",
            "fbo_direct_flow_trans_max_amount",
            "fbo_return_flow_amount",
            "fbs_deliv_to_customer_amount",
            "fbs_direct_flow_trans_min_amount",
            "fbs_direct_flow_trans_max_amount",
            "fbs_first_mile_min_amount",
            "fbs_first_mile_max_amount",
            "fbs_return_flow_amount",
        ] {
            map.insert(key.into(), format_number_cell(c.get(key)));
        }
    }

    if let Some(idx) = item.get("price_indexes") {
        map.insert("color_index".into(), json_str(idx.get("color_index")));
        fill_index(&mut map, "ozon_index", idx.get("ozon_index_data"));
        fill_index(&mut map, "external_index", idx.get("external_index_data"));
        fill_index(
            &mut map,
            "self_marketplaces_index",
            idx.get("self_marketplaces_index_data"),
        );
    }

    if let Some(m) = item.get("marketing_actions") {
        map.insert(
            "ozon_actions_exist".into(),
            yes_no(m.get("ozon_actions_exist").and_then(|x| x.as_bool())),
        );
        map.insert(
            "current_period_from".into(),
            format_ozon_dt(&json_str(m.get("current_period_from"))),
        );
        map.insert(
            "current_period_to".into(),
            format_ozon_dt(&json_str(m.get("current_period_to"))),
        );
        if let Some(actions) = m.get("actions").and_then(|a| a.as_array()) {
            let mut titles = Vec::new();
            let mut values = Vec::new();
            let mut from_d = Vec::new();
            let mut to_d = Vec::new();
            for a in actions {
                titles.push(json_str(a.get("title")));
                values.push(format_number_cell(a.get("value")));
                from_d.push(format_ozon_dt(&json_str(a.get("date_from"))));
                to_d.push(format_ozon_dt(&json_str(a.get("date_to"))));
            }
            map.insert("actions_titles".into(), titles.join(";"));
            map.insert("actions_values".into(), values.join(";"));
            map.insert("actions_date_from".into(), from_d.join(";"));
            map.insert("actions_date_to".into(), to_d.join(";"));
        }
    }

    selected.iter().map(|f| cell(&map, &f.name)).collect()
}

pub fn build_rows_header(fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| f.title.clone())
        .collect()
}
