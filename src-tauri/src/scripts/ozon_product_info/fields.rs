//! Поля Ozon product info (паритет `OZ_ProductInfoScript`).

use crate::core::script_field::ScriptField;
use crate::scripts::ozon_util::{
    fill_from_item_and_source, format_ozon_dt, json_str, row_from_map, sheet_price, yes_no,
};
use serde_json::Value;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_product_info.cabinet"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_product_info.offer_id"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_product_info.product_id"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_product_info.sku"), true),
        ScriptField::new("name", crate::core::i18n::t("fields.ozon_product_info.name"), true),
        ScriptField::new("price", crate::core::i18n::t("fields.ozon_product_info.price"), true),
        ScriptField::new("old_price", crate::core::i18n::t("fields.ozon_product_info.old_price"), true),
        ScriptField::new("min_price", crate::core::i18n::t("fields.ozon_product_info.min_price"), true),
        ScriptField::new("currency_code", crate::core::i18n::t("fields.ozon_product_info.currency_code"), true),
        ScriptField::new("vat", crate::core::i18n::t("fields.ozon_product_info.vat"), true),
        ScriptField::new("is_archived", crate::core::i18n::t("fields.ozon_product_info.is_archived"), true),
        ScriptField::new("is_autoarchived", crate::core::i18n::t("fields.ozon_product_info.is_autoarchived"), true),
        ScriptField::new("is_discounted", crate::core::i18n::t("fields.ozon_product_info.is_discounted"), true),
        ScriptField::new("is_kgt", crate::core::i18n::t("fields.ozon_product_info.is_kgt"), true),
        ScriptField::new("is_super", crate::core::i18n::t("fields.ozon_product_info.is_super"), true),
        ScriptField::new("status", crate::core::i18n::t("fields.ozon_product_info.status"), true),
        ScriptField::new("status_name", crate::core::i18n::t("fields.ozon_product_info.status_name"), true),
        ScriptField::new("status_description", crate::core::i18n::t("fields.ozon_product_info.status_description"), true),
        ScriptField::new("moderate_status", crate::core::i18n::t("fields.ozon_product_info.moderate_status"), true),
        ScriptField::new("validation_status", crate::core::i18n::t("fields.ozon_product_info.validation_status"), true),
        ScriptField::new("has_stock", crate::core::i18n::t("fields.ozon_product_info.has_stock"), true),
        ScriptField::new("discounted_fbo_stocks", crate::core::i18n::t("fields.ozon_product_info.discounted_fbo_stocks"), true),
        ScriptField::new("has_discounted_fbo_item", crate::core::i18n::t("fields.ozon_product_info.has_discounted_fbo_item"), true),
        ScriptField::new("created_at", crate::core::i18n::t("fields.ozon_product_info.created_at"), true),
        ScriptField::new("updated_at", crate::core::i18n::t("fields.ozon_product_info.updated_at"), true),
        ScriptField::new("description_category_id", crate::core::i18n::t("fields.ozon_product_info.description_category_id"), false),
        ScriptField::new("type_id", crate::core::i18n::t("fields.ozon_product_info.type_id"), false),
        ScriptField::new("volume_weight", crate::core::i18n::t("fields.ozon_product_info.volume_weight"), false),
        ScriptField::new("barcodes", crate::core::i18n::t("fields.ozon_product_info.barcodes"), false),
        ScriptField::new("images", crate::core::i18n::t("fields.ozon_product_info.images"), false),
        ScriptField::new("primary_image", crate::core::i18n::t("fields.ozon_product_info.primary_image"), false),
        ScriptField::new("color_image", crate::core::i18n::t("fields.ozon_product_info.color_image"), false),
        ScriptField::new("images360", crate::core::i18n::t("fields.ozon_product_info.images360"), false),
        ScriptField::new("quant_code", crate::core::i18n::t("fields.ozon_product_info.quant_code"), false),
        ScriptField::new("shipment_type", crate::core::i18n::t("fields.ozon_product_info.shipment_type"), false),
        ScriptField::new("source", crate::core::i18n::t("fields.ozon_product_info.source"), false),
        ScriptField::new("model_id", crate::core::i18n::t("fields.ozon_product_info.model_id"), false),
        ScriptField::new("model_name", crate::core::i18n::t("fields.ozon_product_info.model_name"), false),
        ScriptField::new("model_count", crate::core::i18n::t("fields.ozon_product_info.model_count"), false),
        ScriptField::new("price_index_color", crate::core::i18n::t("fields.ozon_product_info.price_index_color"), false),
        ScriptField::new("price_index_value", crate::core::i18n::t("fields.ozon_product_info.price_index_value"), false),
        ScriptField::new("ozon_minimal_price", crate::core::i18n::t("fields.ozon_product_info.ozon_minimal_price"), false),
        ScriptField::new("external_minimal_price", crate::core::i18n::t("fields.ozon_product_info.external_minimal_price"), false),
        ScriptField::new(
            "self_marketplaces_minimal_price",
            "Мин. цена на других площадках",
            false,
        ),
        ScriptField::new("availability", crate::core::i18n::t("fields.ozon_product_info.availability"), false),
        ScriptField::new("availability_reasons", crate::core::i18n::t("fields.ozon_product_info.availability_reasons"), false),
        ScriptField::new("errors", crate::core::i18n::t("fields.ozon_product_info.errors"), false),
        ScriptField::new("source_fbo", crate::core::i18n::t("fields.ozon_product_info.source_fbo"), false),
        ScriptField::new("source_fbs", crate::core::i18n::t("fields.ozon_product_info.source_fbs"), false),
        ScriptField::new("source_rfbs", crate::core::i18n::t("fields.ozon_product_info.source_rfbs"), false),
        ScriptField::new("source_fbp", crate::core::i18n::t("fields.ozon_product_info.source_fbp"), false),
        ScriptField::new("source_created_at", crate::core::i18n::t("fields.ozon_product_info.source_created_at"), false),
        ScriptField::new("promotions", crate::core::i18n::t("fields.ozon_product_info.promotions"), false),
        ScriptField::new("visibility_has_price", crate::core::i18n::t("fields.ozon_product_info.visibility_has_price"), false),
        ScriptField::new("visibility_has_stock", crate::core::i18n::t("fields.ozon_product_info.visibility_has_stock"), false),
    ]
}

fn join_array(item: &Value, key: &str) -> String {
    item.get(key)
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| json_str(Some(v)))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn first_array_str(item: &Value, key: &str) -> String {
    item.get(key)
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .map(|v| json_str(Some(v)))
        .unwrap_or_default()
}

fn resolve_old_price(item: &Value) -> String {
    let direct = json_str(item.get("old_price"));
    if !direct.is_empty() {
        return sheet_price(&direct);
    }
    if let Some(p) = item
        .pointer("/marketing_price/old_price")
        .map(|v| json_str(Some(v)))
        .filter(|s| !s.is_empty())
    {
        return sheet_price(&p);
    }
    for path in [
        "/price_indexes/ozon_index_data/old_price",
        "/price_indexes/external_index_data/old_price",
        "/price_indexes/old_price",
    ] {
        if let Some(p) = item
            .pointer(path)
            .map(|v| json_str(Some(v)))
            .filter(|s| !s.is_empty())
        {
            return sheet_price(&p);
        }
    }
    String::new()
}

fn resolve_name(item: &Value) -> String {
    let n = json_str(item.get("name"));
    if !n.is_empty() {
        return n;
    }
    json_str(item.pointer("/product_info/name"))
}

pub fn build_row(
    item: &Value,
    cabinet: &str,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
) -> Vec<String> {
    let ids = fill_from_item_and_source(item, id_mode, source_id);
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    map.insert("offer_id".into(), ids.offer_id);
    map.insert("product_id".into(), ids.product_id);
    map.insert("sku".into(), ids.sku);
    map.insert("name".into(), resolve_name(item));
    map.insert(
        "price".into(),
        sheet_price(&json_str(item.get("price"))),
    );
    map.insert("old_price".into(), resolve_old_price(item));
    map.insert(
        "min_price".into(),
        sheet_price(&json_str(item.get("min_price"))),
    );
    map.insert("currency_code".into(), json_str(item.get("currency_code")));
    map.insert("vat".into(), json_str(item.get("vat")));
    map.insert(
        "is_archived".into(),
        yes_no(item.get("is_archived").and_then(|v| v.as_bool())),
    );
    map.insert(
        "is_autoarchived".into(),
        yes_no(item.get("is_autoarchived").and_then(|v| v.as_bool())),
    );
    map.insert(
        "is_discounted".into(),
        yes_no(item.get("is_discounted").and_then(|v| v.as_bool())),
    );
    map.insert(
        "is_kgt".into(),
        yes_no(item.get("is_kgt").and_then(|v| v.as_bool())),
    );
    map.insert(
        "is_super".into(),
        yes_no(item.get("is_super").and_then(|v| v.as_bool())),
    );
    map.insert(
        "description_category_id".into(),
        json_str(item.get("description_category_id")),
    );
    map.insert("type_id".into(), json_str(item.get("type_id")));
    map.insert("volume_weight".into(), json_str(item.get("volume_weight")));
    map.insert(
        "created_at".into(),
        format_ozon_dt(&json_str(item.get("created_at"))),
    );
    map.insert(
        "updated_at".into(),
        format_ozon_dt(&json_str(item.get("updated_at"))),
    );
    map.insert("quant_code".into(), json_str(item.get("quant_code")));
    map.insert("shipment_type".into(), json_str(item.get("shipment_type")));
    map.insert("source".into(), String::new());
    map.insert("model_id".into(), String::new());
    map.insert("model_name".into(), String::new());
    map.insert("model_count".into(), String::new());
    map.insert("source_fbo".into(), String::new());
    map.insert("source_fbs".into(), String::new());
    map.insert("source_rfbs".into(), String::new());
    map.insert("source_fbp".into(), String::new());
    map.insert("source_created_at".into(), String::new());
    map.insert("images360".into(), String::new());
    map.insert("status".into(), String::new());
    map.insert("status_name".into(), String::new());
    map.insert("status_description".into(), String::new());
    map.insert("moderate_status".into(), String::new());
    map.insert("validation_status".into(), String::new());
    map.insert("has_stock".into(), "Нет".into());
    map.insert(
        "discounted_fbo_stocks".into(),
        json_str(item.get("discounted_fbo_stocks")),
    );
    map.insert(
        "has_discounted_fbo_item".into(),
        yes_no(
            item.get("has_discounted_fbo_item")
                .and_then(|v| v.as_bool()),
        ),
    );
    map.insert("barcodes".into(), join_array(item, "barcodes"));
    map.insert("images".into(), join_array(item, "images"));
    map.insert("primary_image".into(), first_array_str(item, "primary_image"));
    map.insert("color_image".into(), first_array_str(item, "color_image"));
    map.insert("images360".into(), join_array(item, "images360"));
    map.insert("price_index_color".into(), String::new());
    map.insert("price_index_value".into(), String::new());
    map.insert("ozon_minimal_price".into(), String::new());
    map.insert("external_minimal_price".into(), String::new());
    map.insert("self_marketplaces_minimal_price".into(), String::new());
    map.insert("availability".into(), String::new());
    map.insert("availability_reasons".into(), String::new());
    map.insert("errors".into(), String::new());
    map.insert("promotions".into(), String::new());
    map.insert("visibility_has_price".into(), String::new());
    map.insert("visibility_has_stock".into(), String::new());

    if let Some(st) = item.get("statuses") {
        map.insert("status".into(), json_str(st.get("status")));
        map.insert("status_name".into(), json_str(st.get("status_name")));
        map.insert(
            "status_description".into(),
            json_str(st.get("status_description")),
        );
        map.insert(
            "moderate_status".into(),
            json_str(st.get("moderate_status")),
        );
        map.insert(
            "validation_status".into(),
            json_str(st.get("validation_status")),
        );
    }

    if let Some(hs) = item.pointer("/stocks/has_stock").and_then(|v| v.as_bool()) {
        map.insert("has_stock".into(), yes_no(Some(hs)));
    }

    if let Some(mi) = item.get("model_info") {
        map.insert("model_id".into(), json_str(mi.get("model_id")));
        map.insert("model_name".into(), json_str(mi.get("name")));
        map.insert("model_count".into(), json_str(mi.get("count")));
    }

    if let Some(pi) = item.get("price_indexes") {
        map.insert("price_index_color".into(), json_str(pi.get("color_index")));
        if let Some(oz) = pi.get("ozon_index_data") {
            map.insert(
                "ozon_minimal_price".into(),
                sheet_price(&json_str(oz.get("minimal_price"))),
            );
            map.insert(
                "price_index_value".into(),
                json_str(oz.get("price_index_value")),
            );
        }
        if let Some(ex) = pi.get("external_index_data") {
            map.insert(
                "external_minimal_price".into(),
                sheet_price(&json_str(ex.get("minimal_price"))),
            );
        }
        if let Some(sm) = pi.get("self_marketplaces_index_data") {
            map.insert(
                "self_marketplaces_minimal_price".into(),
                sheet_price(&json_str(sm.get("minimal_price"))),
            );
        }
    }

    if let Some(arr) = item.get("availabilities").and_then(|a| a.as_array()) {
        if let Some(first) = arr.first() {
            map.insert(
                "availability".into(),
                json_str(first.get("availability")),
            );
            if let Some(reasons) = first.get("reasons").and_then(|r| r.as_array()) {
                let texts: Vec<String> = reasons
                    .iter()
                    .filter_map(|r| r.pointer("/human_text/text"))
                    .map(|t| json_str(Some(t)))
                    .filter(|s| !s.is_empty())
                    .collect();
                map.insert("availability_reasons".into(), texts.join("; "));
            }
        }
    }

    if let Some(vd) = item.get("visibility_details") {
        map.insert(
            "visibility_has_price".into(),
            yes_no(vd.get("has_price").and_then(|v| v.as_bool())),
        );
        map.insert(
            "visibility_has_stock".into(),
            yes_no(vd.get("has_stock").and_then(|v| v.as_bool())),
        );
    }

    if let Some(sources) = item.get("sources").and_then(|a| a.as_array()) {
        let mut source_list = Vec::new();
        for src in sources {
            let source = json_str(src.get("source"));
            let created = format_ozon_dt(&json_str(src.get("created_at")));
            if !source.is_empty() {
                source_list.push(source.clone());
            }
            let key = match source.to_ascii_uppercase().as_str() {
                "FBO" => "source_fbo",
                "FBS" => "source_fbs",
                "RFBS" => "source_rfbs",
                "FBP" => "source_fbp",
                _ => "",
            };
            if !key.is_empty() {
                map.insert(key.into(), source.clone());
                if !created.is_empty()
                    && map
                        .get("source_created_at")
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                {
                    map.insert("source_created_at".into(), created);
                }
            }
        }
        if !source_list.is_empty() {
            map.insert("source".into(), source_list.join(", "));
        }
    }

    if let Some(errors) = item.get("errors").and_then(|a| a.as_array()) {
        let texts: Vec<String> = errors
            .iter()
            .filter_map(|e| {
                e.pointer("/texts/message")
                    .or_else(|| e.pointer("/texts/description"))
            })
            .map(|t| json_str(Some(t)))
            .filter(|s| !s.is_empty())
            .collect();
        map.insert("errors".into(), texts.join("; "));
    }

    if let Some(promos) = item.get("promotions").and_then(|a| a.as_array()) {
        let texts: Vec<String> = promos
            .iter()
            .filter(|p| p.get("is_enabled").and_then(|v| v.as_bool()) == Some(true))
            .map(|p| json_str(p.get("type")))
            .filter(|s| !s.is_empty())
            .collect();
        map.insert("promotions".into(), texts.join(", "));
    }

    row_from_map(&map, fields)
}
