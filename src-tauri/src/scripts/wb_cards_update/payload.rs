//! Сборка тела POST /content/v2/cards/update.

use serde_json::{json, Value};

const VAT_CHARC_ID: i64 = 15_001_405;
const OBSOLETE_CHARC: [i64; 4] = [15_001_135, 15_001_136, 15_001_137, 15_001_138];

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

    let old_vc = jstr(card, &["vendorCode"]);
    let vendor_diff = d.vendor && normalize_vendor(&old_vc) != normalize_vendor(&d.vendor_code);

    let old_vat = extract_vat(card);
    let vat_diff = d.vat_mode && normalize_vat(&old_vat) != normalize_vat(&d.vat);

    let old_brand = jstr(card, &["brand"]);
    let brand_diff = d.brand_mode && old_brand.trim() != d.brand.trim();

    dims_diff || vendor_diff || vat_diff || brand_diff
}

pub fn build_item(card: &Value, d: &Desired) -> Value {
    let old_l = dim_i(card, "length");
    let old_w = dim_i(card, "width");
    let old_h = dim_i(card, "height");
    let nm = card
        .get("nmID")
        .or_else(|| card.get("nmId"))
        .cloned()
        .unwrap_or(Value::Null);
    let vendor = if d.vendor {
        d.vendor_code.clone()
    } else {
        jstr(card, &["vendorCode"])
    };
    let brand = if d.brand_mode {
        d.brand.clone()
    } else {
        jstr(card, &["brand"])
    };
    let length = if d.dims { d.length } else { old_l };
    let width = if d.dims { d.width } else { old_w };
    let height = if d.dims { d.height } else { old_h };

    let mut root = json!({
        "nmID": nm,
        "vendorCode": vendor,
        "brand": brand,
        "title": jstr(card, &["title"]),
        "description": jstr(card, &["description"]),
        "dimensions": {
            "length": length,
            "width": width,
            "height": height,
            "weightBrutto": card.get("dimensions").and_then(|x| x.get("weightBrutto")).cloned().unwrap_or(Value::Null)
        }
    });

    if let Some(docs) = card.get("documents") {
        root["documents"] = docs.clone();
    }
    root["characteristics"] = build_characteristics(card, d.vat_mode, &d.vat);
    root["sizes"] = build_sizes(card);
    root
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
        let mut obj = serde_json::Map::new();
        if let Some(id) = s.get("chrtID").or_else(|| s.get("chrtId")) {
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

fn jstr(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(x) = v.get(*k) {
            return match x {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string().trim_matches('"').to_string(),
            };
        }
    }
    String::new()
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
