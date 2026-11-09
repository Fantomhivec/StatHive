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

//! Поля `orders_fbs` / `orders_fbs_status` — schema marketplace v3.

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
        f("officeId", false),
        f("offices", false),
        f("address.fullAddress", false),
        f("address.longitude", false),
        f("address.latitude", false),
        f("nmId", true),
        f("chrtId", false),
        f("article", true),
        f("colorCode", false),
        f("skus", false),
        f("price", false),
        f("convertedPrice", true),
        f("currencyCode", false),
        f("convertedCurrencyCode", false),
        f("cargoType", false),
        f("crossBorderType", false),
        f("isZeroOrder", false),
        f("orderUid", false),
        f("deliveryType", false),
        f("comment", false),
        f("scanPrice", false),
        f("supplyId", false),
        f("trackNumber", false),
        f("options.isB2B", false),
    ]
}

pub fn status_fields() -> Vec<ScriptField> {
    vec![
        f_status("cabinet", true),
        f_status("id", true),
        f_status("supplierStatus", true),
        f_status("wbStatus", true),
        f_status("isCancellable", false),
        f_status("isTransferable", false),
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
        "offices" | "skus" => join_str_array(o, name),
        "address.fullAddress" => dig(o, &["address", "fullAddress"]),
        "address.longitude" => dig(o, &["address", "longitude"]),
        "address.latitude" => dig(o, &["address", "latitude"]),
        "options.isB2B" => dig(o, &["options", "isB2B"]),
        _ => any(o, name),
    }
}

fn status_cell(cabinet: &str, o: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        _ => any(o, name),
    }
}

fn join_str_array(o: &Value, key: &str) -> String {
    o.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

fn dig(o: &Value, path: &[&str]) -> String {
    let mut cur = o;
    for p in path {
        cur = match cur.get(*p) {
            Some(x) => x,
            None => return String::new(),
        };
    }
    match cur {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
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
