use crate::core::script_field::ScriptField;
use crate::scripts::ozon_util::{join_str_array, json_str};
use serde_json::Value;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_product_attributes.cabinet"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_product_attributes.product_id"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_product_attributes.offer_id"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_product_attributes.sku"), true),
        ScriptField::new("name", crate::core::i18n::t("fields.ozon_product_attributes.name"), true),
        ScriptField::new("barcode", crate::core::i18n::t("fields.ozon_product_attributes.barcode"), true),
        ScriptField::new("barcodes", crate::core::i18n::t("fields.ozon_product_attributes.barcodes"), false),
        ScriptField::new("description_category_id", crate::core::i18n::t("fields.ozon_product_attributes.description_category_id"), false),
        ScriptField::new("color_image", crate::core::i18n::t("fields.ozon_product_attributes.color_image"), false),
        ScriptField::new("depth", crate::core::i18n::t("fields.ozon_product_attributes.depth"), false),
        ScriptField::new("height", crate::core::i18n::t("fields.ozon_product_attributes.height"), false),
        ScriptField::new("width", crate::core::i18n::t("fields.ozon_product_attributes.width"), false),
        ScriptField::new("weight", crate::core::i18n::t("fields.ozon_product_attributes.weight"), false),
        ScriptField::new("dimension_unit", crate::core::i18n::t("fields.ozon_product_attributes.dimension_unit"), false),
        ScriptField::new("weight_unit", crate::core::i18n::t("fields.ozon_product_attributes.weight_unit"), false),
        ScriptField::new("type_id", crate::core::i18n::t("fields.ozon_product_attributes.type_id"), false),
        ScriptField::new("primary_image", crate::core::i18n::t("fields.ozon_product_attributes.primary_image"), false),
        ScriptField::new("images", crate::core::i18n::t("fields.ozon_product_attributes.images"), false),
        ScriptField::new("model_id", crate::core::i18n::t("fields.ozon_product_attributes.model_id"), false),
        ScriptField::new("model_name", crate::core::i18n::t("fields.ozon_product_attributes.model_name"), false),
        ScriptField::new("model_count", crate::core::i18n::t("fields.ozon_product_attributes.model_count"), false),
        ScriptField::new("brand_attribute", crate::core::i18n::t("fields.ozon_product_attributes.brand_attribute"), true),
        ScriptField::new("tnved_attribute", crate::core::i18n::t("fields.ozon_product_attributes.tnved_attribute"), true),
        ScriptField::new("attributes", crate::core::i18n::t("fields.ozon_product_attributes.attributes"), false),
        ScriptField::new(
            "attributes_with_defaults",
            "ID характеристик со значением по умолчанию",
            false,
        ),
        ScriptField::new("complex_attributes", crate::core::i18n::t("fields.ozon_product_attributes.complex_attributes"), false),
        ScriptField::new("pdf_list", crate::core::i18n::t("fields.ozon_product_attributes.pdf_list"), false),
    ]
}

fn cell(map: &HashMap<String, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_row(
    cabinet: &str,
    item: &Value,
    fields: &[ScriptField],
    id_mode: &str,
    source_id: Option<&str>,
) -> Vec<String> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();

    let mut product_id = json_str(item.get("id").or_else(|| item.get("product_id")));
    let mut offer_id = json_str(item.get("offer_id"));
    let mut sku = json_str(item.get("sku"));
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
    map.insert("product_id".into(), product_id);
    map.insert("offer_id".into(), offer_id);
    map.insert("sku".into(), sku);
    map.insert("name".into(), json_str(item.get("name")));
    map.insert("barcode".into(), json_str(item.get("barcode")));
    map.insert(
        "barcodes".into(),
        join_str_array(item.get("barcodes"), ", "),
    );
    map.insert(
        "description_category_id".into(),
        json_str(item.get("description_category_id")),
    );
    map.insert("color_image".into(), json_str(item.get("color_image")));
    map.insert("depth".into(), json_str(item.get("depth")));
    map.insert("height".into(), json_str(item.get("height")));
    map.insert("width".into(), json_str(item.get("width")));
    map.insert("weight".into(), json_str(item.get("weight")));
    map.insert("dimension_unit".into(), json_str(item.get("dimension_unit")));
    map.insert("weight_unit".into(), json_str(item.get("weight_unit")));
    map.insert("type_id".into(), json_str(item.get("type_id")));
    map.insert("primary_image".into(), json_str(item.get("primary_image")));
    map.insert("images".into(), join_str_array(item.get("images"), ", "));

    if let Some(m) = item.get("model_info") {
        map.insert("model_id".into(), json_str(m.get("model_id")));
        map.insert("model_name".into(), json_str(m.get("name")));
        map.insert("model_count".into(), json_str(m.get("count")));
    }

    let mut brand = String::new();
    let mut tnved = String::new();
    if let Some(attrs) = item.get("attributes").and_then(|a| a.as_array()) {
        for a in attrs {
            let id = a.get("id").and_then(|x| x.as_i64());
            let val = a
                .get("values")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|o| o.get("value"))
                .map(|v| json_str(Some(v)))
                .unwrap_or_default();
            if id == Some(85) {
                brand = val.clone();
            } else if id == Some(22232) {
                tnved = val;
            }
        }
        map.insert(
            "attributes".into(),
            serde_json::to_string(attrs).unwrap_or_default(),
        );
    }
    map.insert("brand_attribute".into(), brand);
    map.insert("tnved_attribute".into(), tnved);

    map.insert(
        "attributes_with_defaults".into(),
        join_str_array(item.get("attributes_with_defaults"), ", "),
    );
    if let Some(c) = item.get("complex_attributes") {
        map.insert(
            "complex_attributes".into(),
            serde_json::to_string(c).unwrap_or_default(),
        );
    }
    if let Some(p) = item.get("pdf_list") {
        map.insert(
            "pdf_list".into(),
            serde_json::to_string(p).unwrap_or_default(),
        );
    }

    selected.iter().map(|f| cell(&map, &f.name)).collect()
}
