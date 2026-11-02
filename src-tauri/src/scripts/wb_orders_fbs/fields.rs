//! Поля `orders_fbs` / `orders_fbs_status`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn orders_fields() -> Vec<ScriptField> {
    vec![
        f("cabinet", true),
        f("id", true),
        f("rid", false),
        f("createdAt", true),
        f("warehouseId", false),
        f("offices", false),
        f("address", false),
        f("userStatus", false),
        f("nmId", true),
        f("chrtId", false),
        f("article", true),
        f("convertedPrice", true),
        f("currencyCode", false),
        f("price", false),
        f("salePrice", false),
        f("isZeroOrder", false),
        f("orderUid", false),
        f("deliveryType", false),
        f("comment", false),
        f("scanPrice", false),
        f("supplyId", false),
    ]
}

pub fn status_fields() -> Vec<ScriptField> {
    vec![
        f_status("cabinet", true),
        f_status("id", true),
        f_status("supplierStatus", true),
        f_status("wbStatus", true),
    ]
}

fn f(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.orders_fbs.{name}")),
        selected,
    }
}

fn f_status(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.orders_fbs_status.{name}")),
        selected,
    }
}

pub fn build_order_rows(cabinet: &str, orders: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut out = Vec::with_capacity(orders.len() + 1);
    out.push(selected.iter().map(|f| f.title.clone()).collect());
    for o in orders {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(order_cell(cabinet, o, f.name.as_str()));
        }
        out.push(row);
    }
    out
}

pub fn build_status_rows(cabinet: &str, items: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut out = Vec::with_capacity(items.len() + 1);
    out.push(selected.iter().map(|f| f.title.clone()).collect());
    for o in items {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(status_cell(cabinet, o, f.name.as_str()));
        }
        out.push(row);
    }
    out
}

fn order_cell(cabinet: &str, o: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "createdAt" => {
            let raw = str_any(o, "createdAt");
            if raw.is_empty() {
                raw
            } else {
                sheet_dates::format_wb_timestamp(&raw)
            }
        }
        "offices" => o
            .get("offices")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default(),
        _ => any(o, name),
    }
}

fn status_cell(cabinet: &str, o: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        _ => any(o, name),
    }
}

fn any(o: &Value, key: &str) -> String {
    match o.get(key) {
        Some(Value::Null) | None => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn str_any(o: &Value, key: &str) -> String {
    match o.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}
