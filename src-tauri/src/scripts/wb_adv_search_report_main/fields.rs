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

//! Поля `wb_adv_search_report_main` — flatten `data.groups[].items[]`.

use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("subjectName", false),
        field("subjectId", false),
        field("brandName", true),
        field("tagName", false),
        field("tagId", false),
        field("nmId", true),
        field("name", true),
        field("vendorCode", true),
        field("mainPhoto", false),
        field("isAdvertised", false),
        field("isSubstitutedSKU", false),
        field("isCardRated", false),
        field("rating", false),
        field("feedbackRating", false),
        field("avgPosition", true),
        field("visibility", true),
        field("openCard", true),
        field("addToCart", true),
        field("openToCart", true),
        field("orders", true),
        field("cartToOrder", true),
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
    let groups = root
        .pointer("/data/groups")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    for g in &groups {
        let g_subject = json_str(g, "subjectName");
        let g_subject_id = json_num(g, "subjectId");
        let g_brand = json_str(g, "brandName");
        let g_tag = json_str(g, "tagName");
        let g_tag_id = json_num(g, "tagId");
        let items = g
            .get("items")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            // Group-level metrics only.
            out.push(row(
                &selected,
                cabinet,
                &g_subject,
                &g_subject_id,
                &g_brand,
                &g_tag,
                &g_tag_id,
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                g.get("metrics").unwrap_or(g),
            ));
            continue;
        }
        for it in &items {
            out.push(row(
                &selected,
                cabinet,
                &coalesce(json_str(it, "subjectName"), &g_subject),
                &coalesce(json_num(it, "subjectId"), &g_subject_id),
                &coalesce(json_str(it, "brandName"), &g_brand),
                &g_tag,
                &g_tag_id,
                &json_num(it, "nmId"),
                &json_str(it, "name"),
                &json_str(it, "vendorCode"),
                &json_str(it, "mainPhoto"),
                &json_bool(it, "isAdvertised"),
                &json_bool(it, "isSubstitutedSKU"),
                &json_bool(it, "isCardRated"),
                &json_num(it, "rating"),
                &json_num(it, "feedbackRating"),
                it,
            ));
        }
    }
}

fn coalesce(primary: String, fallback: &str) -> String {
    if primary.is_empty() {
        fallback.to_string()
    } else {
        primary
    }
}

fn row(
    selected: &[&ScriptField],
    cabinet: &str,
    subject_name: &str,
    subject_id: &str,
    brand_name: &str,
    tag_name: &str,
    tag_id: &str,
    nm_id: &str,
    name: &str,
    vendor_code: &str,
    main_photo: &str,
    is_advertised: &str,
    is_subst: &str,
    is_rated: &str,
    rating: &str,
    feedback_rating: &str,
    metrics_src: &Value,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "subjectName" => subject_name.to_string(),
            "subjectId" => subject_id.to_string(),
            "brandName" => brand_name.to_string(),
            "tagName" => tag_name.to_string(),
            "tagId" => tag_id.to_string(),
            "nmId" => nm_id.to_string(),
            "name" => name.to_string(),
            "vendorCode" => vendor_code.to_string(),
            "mainPhoto" => main_photo.to_string(),
            "isAdvertised" => is_advertised.to_string(),
            "isSubstitutedSKU" => is_subst.to_string(),
            "isCardRated" => is_rated.to_string(),
            "rating" => rating.to_string(),
            "feedbackRating" => feedback_rating.to_string(),
            "avgPosition" => metric_current(metrics_src, "avgPosition"),
            "visibility" => metric_current(metrics_src, "visibility"),
            "openCard" => metric_current(metrics_src, "openCard"),
            "addToCart" => metric_current(metrics_src, "addToCart"),
            "openToCart" => metric_current(metrics_src, "openToCart"),
            "orders" => metric_current(metrics_src, "orders"),
            "cartToOrder" => metric_current(metrics_src, "cartToOrder"),
            _ => String::new(),
        })
        .collect()
}

fn metric_current(v: &Value, key: &str) -> String {
    // Item-level: key.current; group-level: metrics.key.current
    if let Some(n) = v.pointer(&format!("/{key}/current")) {
        return json_any(n);
    }
    if let Some(n) = v.pointer(&format!("/metrics/{key}/current")) {
        return json_any(n);
    }
    String::new()
}

fn json_any(v: &Value) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_num(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn json_bool(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}
