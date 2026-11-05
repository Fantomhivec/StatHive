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

//! Поля выгрузки `wb_supplier_orders` (Statistics API supplier/orders).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField::new(
        name,
        crate::core::i18n::t(&format!("fields.wb_supplier_orders.{name}")),
        selected,
    )
}

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("date", true),
        field("lastChangeDate", true),
        field("warehouseName", true),
        field("warehouseType", false),
        field("countryName", false),
        field("oblastOkrugName", false),
        field("regionName", false),
        field("supplierArticle", true),
        field("nmId", true),
        field("barcode", false),
        field("category", false),
        field("subject", false),
        field("brand", false),
        field("techSize", false),
        field("incomeID", false),
        field("isSupply", false),
        field("isRealization", false),
        field("totalPrice", true),
        field("discountPercent", false),
        field("spp", false),
        field("finishedPrice", true),
        field("priceWithDisc", true),
        field("isCancel", true),
        field("cancelDate", false),
        field("sticker", false),
        field("gNumber", false),
        field("srid", true),
    ]
}

/// Merge template selected_fields with catalog defaults (fix titles; drop unknown names).
pub fn normalize_fields(saved: &[ScriptField]) -> Vec<ScriptField> {
    let defaults = default_fields();
    let by_name: std::collections::HashMap<&str, &ScriptField> =
        defaults.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut out = Vec::with_capacity(saved.len());
    for f in saved {
        let Some(def) = by_name.get(f.name.as_str()) else {
            continue;
        };
        out.push(ScriptField::new(
            f.name.as_str(),
            def.title.clone(),
            f.selected,
        ));
    }
    if out.is_empty() {
        return defaults;
    }
    out
}

pub fn build_rows(cabinet: &str, orders: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut out = Vec::with_capacity(orders.len() + 1);
    out.push(selected.iter().map(|f| f.title.clone()).collect());
    for o in orders {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(field_value(cabinet, o, &f.name));
        }
        out.push(row);
    }
    out
}

/// One-shot diagnostic when price columns are empty on the first data row.
pub fn price_fields_empty_diag(orders: &[Value], fields: &[ScriptField]) -> Option<String> {
    let price_names = ["totalPrice", "spp", "finishedPrice", "priceWithDisc", "discountPercent"];
    let selected_prices: Vec<&str> = fields
        .iter()
        .filter(|f| f.selected && price_names.contains(&f.name.as_str()))
        .map(|f| f.name.as_str())
        .collect();
    if selected_prices.is_empty() {
        return None;
    }
    let Some(o) = orders.first() else {
        return None;
    };
    let all_empty = selected_prices
        .iter()
        .all(|n| field_value("", o, n).is_empty());
    if !all_empty {
        return None;
    }
    let keys: Vec<&str> = o
        .as_object()
        .map(|m| m.keys().map(|k| k.as_str()).collect())
        .unwrap_or_default();
    Some(format!(
        "Пустые price-поля на первой строке; ключи JSON заказа: {}",
        keys.join(", ")
    ))
}

fn field_value(cabinet: &str, o: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "date" | "lastChangeDate" => sheet_dates::format_wb_timestamp(&json_str(o.get(name))),
        "cancelDate" => format_cancel_date(&json_str(o.get("cancelDate"))),
        "warehouseName" | "warehouseType" | "countryName" | "oblastOkrugName" | "regionName"
        | "supplierArticle" | "barcode" | "category" | "subject" | "brand" | "techSize"
        | "sticker" | "gNumber" | "srid" => json_str(o.get(name)),
        "nmId" | "incomeID" => num_str(o.get(name)),
        "discountPercent" | "totalPrice" | "spp" | "finishedPrice" | "priceWithDisc" => {
            format_number_ru(o.get(name))
        }
        "isSupply" | "isRealization" | "isCancel" => format_bool(o.get(name)),
        _ => String::new(),
    }
}

fn format_cancel_date(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty()
        || s.starts_with("0001-01-01")
        || s.starts_with("2001-01-01")
    {
        return String::new();
    }
    sheet_dates::format_wb_timestamp(s)
}

fn json_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

fn num_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Bool(b)) => {
            if *b {
                "1".into()
            } else {
                "0".into()
            }
        }
        _ => String::new(),
    }
}

fn format_bool(v: Option<&Value>) -> String {
    match v {
        Some(Value::Bool(true)) => "Да".into(),
        Some(Value::Bool(false)) => "Нет".into(),
        Some(Value::Number(n)) => {
            if n.as_f64() == Some(0.0) {
                "Нет".into()
            } else if n.as_f64().is_some() {
                "Да".into()
            } else {
                String::new()
            }
        }
        Some(Value::String(s)) => {
            let t = s.trim().to_ascii_lowercase();
            match t.as_str() {
                "true" | "1" | "yes" | "да" => "Да".into(),
                "false" | "0" | "no" | "нет" => "Нет".into(),
                _ => String::new(),
            }
        }
        _ => String::new(),
    }
}

fn format_number_ru(v: Option<&Value>) -> String {
    let n = match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim().replace(',', ".");
            t.parse::<f64>().ok()
        }
        _ => None,
    };
    let Some(n) = n else {
        return String::new();
    };
    format!("{n:.2}").replace('.', ",")
}
