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

use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmID", true),
        field("vendorCode", true),
        field("discount", true),
        field("clubDiscount", false),
        field("currencyIsoCode4217", true),
        field("editableSizePrice", false),
        field("price", true),
        field("discountedPrice", true),
        field("clubDiscountedPrice", false),
        field("sizeID", false),
        field("techSizeName", false),
        field("sizesCount", false),
        field("minPrice", false),
        field("maxPrice", false),
        field("minDiscountedPrice", false),
        field("maxDiscountedPrice", false),
        field("sizesPriceList", false),
        field("sizesTechSizeList", false),
        field("wholesaleLevels", false),
        field("wholesaleMinQuantities", false),
        field("wholesaleDiscounts", false),
    ]
}

/// Поля `prices_by_nm` — те же schema + wholesale*.
pub fn default_fields_by_nm() -> Vec<ScriptField> {
    default_fields()
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.prices_export.{name}")),
        selected,
    }
}

pub fn build_rows(cabinet: &str, goods: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = Vec::with_capacity(goods.len() + 1);
    rows.push(selected.iter().map(|f| f.title.clone()).collect());
    for it in goods {
        let sizes = it
            .get("sizes")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();
        let sz = sizes.first();
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), cabinet, it, sz, &sizes));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, cabinet: &str, it: &Value, sz: Option<&Value>, sizes: &[Value]) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmID" => num(it, "nmID"),
        "vendorCode" => str(it, "vendorCode"),
        "discount" => num(it, "discount"),
        "clubDiscount" => num(it, "clubDiscount"),
        "currencyIsoCode4217" => str(it, "currencyIsoCode4217"),
        "editableSizePrice" => bool_str(it, "editableSizePrice"),
        "price" => sz.map(|s| num(s, "price")).unwrap_or_else(|| "0".into()),
        "discountedPrice" => sz
            .map(|s| num(s, "discountedPrice"))
            .unwrap_or_else(|| "0".into()),
        "clubDiscountedPrice" => sz
            .map(|s| num(s, "clubDiscountedPrice"))
            .unwrap_or_else(|| "0".into()),
        "sizeID" => sz.map(|s| num(s, "sizeID")).unwrap_or_else(|| "0".into()),
        "techSizeName" => sz
            .map(|s| str(s, "techSizeName"))
            .unwrap_or_default(),
        "sizesCount" => sizes.len().to_string(),
        "minPrice" => min_max_num(sizes, "price", true),
        "maxPrice" => min_max_num(sizes, "price", false),
        "minDiscountedPrice" => min_max_num(sizes, "discountedPrice", true),
        "maxDiscountedPrice" => min_max_num(sizes, "discountedPrice", false),
        "sizesPriceList" => sizes
            .iter()
            .map(|s| num(s, "price"))
            .collect::<Vec<_>>()
            .join(";"),
        "sizesTechSizeList" => sizes
            .iter()
            .map(|s| str(s, "techSizeName"))
            .collect::<Vec<_>>()
            .join(";"),
        "wholesaleLevels" => wholesale_list(it, "level"),
        "wholesaleMinQuantities" => wholesale_list(it, "minQuantity"),
        "wholesaleDiscounts" => wholesale_list(it, "wholesaleDiscount"),
        _ => String::new(),
    }
}

fn wholesale_list(it: &Value, field_key: &str) -> String {
    it.get("wholesaleDiscountThreshold")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .map(|x| num(x, field_key))
                .collect::<Vec<_>>()
                .join(";")
        })
        .unwrap_or_default()
}

fn str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn num(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn bool_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Bool(b)) => b.to_string(),
        _ => "false".into(),
    }
}

fn min_max_num(sizes: &[Value], key: &str, min: bool) -> String {
    let mut vals: Vec<f64> = sizes
        .iter()
        .filter_map(|s| s.get(key))
        .filter_map(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .collect();
    if vals.is_empty() {
        return "0".into();
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let v = if min {
        *vals.first().unwrap()
    } else {
        *vals.last().unwrap()
    };
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        v.to_string()
    }
}
