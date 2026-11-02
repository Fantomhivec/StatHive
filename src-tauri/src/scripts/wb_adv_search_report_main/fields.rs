//! Поля `wb_adv_search_report_main` (главная страница search-report).

use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("vendorCode", true),
        field("name", true),
        field("brand", true),
        field("subject", true),
        field("avgPosition", true),
        field("visibility", true),
        field("openCard", true),
        field("addToCart", true),
        field("orders", true),
        field("openToCart", true),
        field("cartToOrder", true),
        field("frequency", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_search_report_main.{name}")),
        selected,
    }
}

pub fn append_report_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let data = root.get("data").unwrap_or(root);
    let groups = data
        .get("groups")
        .or_else(|| data.get("products"))
        .or_else(|| data.get("items"))
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    for g in &groups {
        let nm_id = first_num(g, &["nmId", "nmID", "nm_id"]);
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "nmId" => nm_id.clone(),
                    "vendorCode" => first_str(g, &["vendorCode", "vendor_code", "sa_name"]),
                    "name" => first_str(g, &["name", "title"]),
                    "brand" => first_str(g, &["brand", "brandName"]),
                    "subject" => first_str(g, &["subject", "subjectName"]),
                    "avgPosition" => metric(g, &["avgPosition", "avg_position"]),
                    "visibility" => metric(g, &["visibility"]),
                    "openCard" => metric(g, &["openCard", "open_card"]),
                    "addToCart" => metric(g, &["addToCart", "add_to_cart"]),
                    "orders" => metric(g, &["orders"]),
                    "openToCart" => metric(g, &["openToCart", "open_to_cart"]),
                    "cartToOrder" => metric(g, &["cartToOrder", "cart_to_order"]),
                    "frequency" => metric(g, &["frequency", "freq"]),
                    _ => String::new(),
                })
                .collect(),
        );
    }
}

fn metric(v: &Value, keys: &[&str]) -> String {
    if let Some(m) = v.get("metrics").or_else(|| v.get("current")) {
        let s = first_num(m, keys);
        if !s.is_empty() {
            return s;
        }
    }
    first_num(v, keys)
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(Value::String(s)) = v.get(*k) {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    String::new()
}

fn first_num(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match v.get(*k) {
            Some(Value::Number(n)) => return n.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() => return s.clone(),
            _ => {}
        }
    }
    String::new()
}
