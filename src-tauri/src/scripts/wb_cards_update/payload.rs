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

//! Сборка тела POST /content/v2/cards/update.

use serde_json::{json, Map, Value};

const VAT_CHARC_ID: i64 = 15_001_405;
const OBSOLETE_CHARC: [i64; 4] = [15_001_135, 15_001_136, 15_001_137, 15_001_138];

const DOC_ITEM_KEYS: &[&str] = &[
    "id",
    "type",
    "number",
    "productNumber",
    "tradeName",
    "applicant",
    "startDate",
    "endDate",
    "isEndless",
];

pub struct Desired {
    pub length: i32,
    pub width: i32,
    pub height: i32,
    pub vendor_code: String,
    pub vat: String,
    pub brand: String,
    pub dims: bool,
    pub vendor: bool,
    pub vat_mode: bool,
    pub brand_mode: bool,
}

pub fn needs_update(card: &Value, d: &Desired) -> bool {
    let old_l = dim_i(card, "length");
    let old_w = dim_i(card, "width");
    let old_h = dim_i(card, "height");
    let dims_diff = d.dims && (old_l != d.length || old_w != d.width || old_h != d.height);

    let old_vc = jstr(card, "vendorCode");
    let vendor_diff = d.vendor && normalize_vendor(&old_vc) != normalize_vendor(&d.vendor_code);

    let old_vat = extract_vat(card);
    let vat_diff = d.vat_mode && normalize_vat(&old_vat) != normalize_vat(&d.vat);

    let old_brand = jstr(card, "brand");
    let brand_diff = d.brand_mode && old_brand.trim() != d.brand.trim();

    dims_diff || vendor_diff || vat_diff || brand_diff
}

pub fn build_item(card: &Value, d: &Desired) -> Value {
    let old_l = dim_i(card, "length");
    let old_w = dim_i(card, "width");
    let old_h = dim_i(card, "height");
    let nm = card.get("nmID").cloned().unwrap_or(Value::Null);
    let vendor = if d.vendor {
        d.vendor_code.clone()
    } else {
        jstr(card, "vendorCode")
    };
    let brand = if d.brand_mode {
        d.brand.clone()
    } else {
        jstr(card, "brand")
    };
    let length = if d.dims { d.length } else { old_l };
    let width = if d.dims { d.width } else { old_w };
    let height = if d.dims { d.height } else { old_h };

    let mut root = json!({
        "nmID": nm,
        "vendorCode": vendor,
        "brand": brand,
        "title": jstr(card, "title"),
        "description": jstr(card, "description"),
        "dimensions": {
            "length": length,
            "width": width,
            "height": height,
            "weightBrutto": card.get("dimensions").and_then(|x| x.get("weightBrutto")).cloned().unwrap_or(Value::Null)
        }
    });

    // Full rewrite: keep KIZ / GTIN from list card when present.
    if let Some(k) = card.get("kizMarked") {
        if !k.is_null() {
            root["kizMarked"] = k.clone();
        }
    }
    if let Some(g) = card.get("gtin") {
        if !g.is_null() {
            let s = match g {
                Value::String(t) => t.clone(),
                other => other.to_string().trim_matches('"').to_string(),
            };
            if !s.is_empty() {
                root["gtin"] = Value::String(s);
            }
        }
    }

    if let Some(docs) = sanitize_documents(card.get("documents")) {
        root["documents"] = docs;
    }
    root["characteristics"] = build_characteristics(card, d.vat_mode, &d.vat);
    root["sizes"] = build_sizes(card);
    root
}

/// Request schema only: excludeDocuments + items fields (no overallVerdict / verdict).
fn sanitize_documents(docs: Option<&Value>) -> Option<Value> {
    let docs = docs?;
    if docs.is_null() {
        return None;
    }
    let mut out = Map::new();
    if let Some(ex) = docs.get("excludeDocuments") {
        if !ex.is_null() {
            out.insert("excludeDocuments".into(), ex.clone());
        }
    }
    if let Some(items) = docs.get("items").and_then(|i| i.as_array()) {
        let cleaned: Vec<Value> = items
            .iter()
            .map(|item| {
                let mut m = Map::new();
                for k in DOC_ITEM_KEYS {
                    if let Some(v) = item.get(*k) {
                        if !v.is_null() {
                            m.insert((*k).to_string(), v.clone());
                        }
                    }
                }
                Value::Object(m)
            })
            .collect();
        out.insert("items".into(), Value::Array(cleaned));
    }
    if out.is_empty() {
        None
    } else {
        Some(Value::Object(out))
    }
}

fn build_characteristics(card: &Value, vat_mode: bool, vat_value: &str) -> Value {
    let mut arr = Vec::new();
    let mut has_vat = false;
    if let Some(chs) = card.get("characteristics").and_then(|c| c.as_array()) {
        for ch in chs {
            let id = ch.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            if OBSOLETE_CHARC.contains(&id) {
                continue;
            }
            let name = ch.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let is_vat = id == VAT_CHARC_ID || name.eq_ignore_ascii_case("Ставка НДС");
            let mut value = ch.get("value").cloned().unwrap_or(json!([]));
            if vat_mode && is_vat && !vat_value.trim().is_empty() {
                value = normalize_charc_value(vat_value);
                has_vat = true;
            }
            arr.push(json!({ "id": id, "value": value }));
        }
    }
    if vat_mode && !vat_value.trim().is_empty() && !has_vat {
        arr.push(json!({
            "id": VAT_CHARC_ID,
            "value": normalize_charc_value(vat_value)
        }));
    }
    Value::Array(arr)
}

fn build_sizes(card: &Value) -> Value {
    let Some(sizes) = card.get("sizes").and_then(|s| s.as_array()) else {
        return json!([]);
    };
    let mut out = Vec::new();
    for s in sizes {
        let mut obj = Map::new();
        if let Some(id) = s.get("chrtID") {
            if id.as_i64().unwrap_or(0) > 0 {
                obj.insert("chrtID".into(), id.clone());
            }
        }
        if let Some(Value::String(t)) = s.get("techSize") {
            if !t.is_empty() {
                obj.insert("techSize".into(), Value::String(t.clone()));
            }
        }
        if let Some(Value::String(w)) = s.get("wbSize") {
            if !w.is_empty() {
                obj.insert("wbSize".into(), Value::String(w.clone()));
            }
        }
        if let Some(skus) = s.get("skus") {
            obj.insert("skus".into(), skus.clone());
        }
        out.push(Value::Object(obj));
    }
    Value::Array(out)
}

fn normalize_charc_value(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() {
        json!([])
    } else {
        json!([t])
    }
}

fn extract_vat(card: &Value) -> String {
    let Some(chs) = card.get("characteristics").and_then(|c| c.as_array()) else {
        return String::new();
    };
    for ch in chs {
        let id = ch.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
        let name = ch.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if id == VAT_CHARC_ID || name.eq_ignore_ascii_case("Ставка НДС") {
            return match ch.get("value") {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
        }
    }
    String::new()
}

fn dim_i(card: &Value, key: &str) -> i32 {
    card.get("dimensions")
        .and_then(|d| d.get(key))
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0) as i32
}

fn jstr(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
}

fn normalize_vendor(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

fn normalize_vat(s: &str) -> String {
    let t = s.trim();
    if let Ok(n) = t.parse::<i32>() {
        return n.to_string();
    }
    t.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_card() -> Value {
        json!({
            "nmID": 11111111,
            "vendorCode": "old-vc",
            "brand": "OldBrand",
            "title": "Title",
            "description": "Desc",
            "kizMarked": true,
            "gtin": "04607009520001",
            "dimensions": {
                "length": 10,
                "width": 20,
                "height": 30,
                "weightBrutto": 1.5
            },
            "documents": {
                "excludeDocuments": false,
                "overallVerdict": { "status": "ok", "isFullyChecked": true },
                "items": [{
                    "id": "doc-1",
                    "type": 1,
                    "number": "123",
                    "verdict": { "status": "ok" },
                    "startDate": "2023-01-15T03:00:00+03:00",
                    "endDate": "2026-01-14T03:00:00+03:00",
                    "isEndless": false
                }]
            },
            "characteristics": [
                { "id": 50, "name": "Крой", "value": ["свободный"] },
                { "id": 15001405, "name": "Ставка НДС", "value": ["10"] }
            ],
            "sizes": [{
                "chrtID": 123,
                "techSize": "ONE SIZE",
                "wbSize": "78-90",
                "skus": ["sku1"]
            }]
        })
    }

    fn dims_desired() -> Desired {
        Desired {
            length: 35,
            width: 40,
            height: 15,
            vendor_code: String::new(),
            vat: String::new(),
            brand: String::new(),
            dims: true,
            vendor: false,
            vat_mode: false,
            brand_mode: false,
        }
    }

    #[test]
    fn keeps_kiz_and_gtin_from_card() {
        let item = build_item(&base_card(), &dims_desired());
        assert_eq!(item["kizMarked"], json!(true));
        assert_eq!(item["gtin"], json!("04607009520001"));
    }

    #[test]
    fn documents_drop_response_only_fields() {
        let item = build_item(&base_card(), &dims_desired());
        let docs = &item["documents"];
        assert!(docs.get("overallVerdict").is_none());
        assert_eq!(docs["excludeDocuments"], json!(false));
        let item0 = &docs["items"][0];
        assert_eq!(item0["id"], json!("doc-1"));
        assert_eq!(item0["type"], json!(1));
        assert_eq!(item0["number"], json!("123"));
        assert!(item0.get("verdict").is_none());
    }

    #[test]
    fn vat_mode_updates_charc() {
        let mut d = dims_desired();
        d.dims = false;
        d.vat_mode = true;
        d.vat = "20".into();
        let item = build_item(&base_card(), &d);
        let vat = item["characteristics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == 15001405)
            .expect("vat charc");
        assert_eq!(vat["value"], json!(["20"]));
    }

    #[test]
    fn dims_only_keeps_vendor() {
        let item = build_item(&base_card(), &dims_desired());
        assert_eq!(item["vendorCode"], json!("old-vc"));
        assert_eq!(item["dimensions"]["length"], json!(35));
        assert_eq!(item["dimensions"]["weightBrutto"], json!(1.5));
    }

    #[test]
    fn ignores_legacy_nm_id_key() {
        let card = json!({
            "nmId": 999,
            "vendorCode": "x",
            "dimensions": { "length": 1, "width": 1, "height": 1 }
        });
        let item = build_item(&card, &dims_desired());
        assert!(item.get("nmID").map(|v| v.is_null()).unwrap_or(true));
    }
}
