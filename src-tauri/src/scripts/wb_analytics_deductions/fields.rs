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

//! Поля `analytics_deductions` — schema `data.reports[]`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("dtBonus", true),
        field("nmId", true),
        field("bonusSumm", true),
        field("bonusType", true),
        field("oldShkId", true),
        field("oldColor", true),
        field("oldSize", true),
        field("oldSku", true),
        field("oldVendorCode", true),
        field("newShkId", true),
        field("newColor", true),
        field("newSize", true),
        field("newSku", true),
        field("newVendorCode", true),
        field("photoUrls", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.analytics_deductions.{name}")),
        selected,
    }
}

pub fn append_rows(
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
    let reports = data
        .get("reports")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    for d in &reports {
        let photos = d
            .get("photoUrls")
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "dtBonus" => fmt_ts(json_str(d, "dtBonus")),
                    "nmId" => json_num(d, "nmId"),
                    "bonusSumm" => json_num(d, "bonusSumm"),
                    "bonusType" => json_str(d, "bonusType"),
                    "oldShkId" => json_num(d, "oldShkId"),
                    "oldColor" => json_str(d, "oldColor"),
                    "oldSize" => json_str(d, "oldSize"),
                    "oldSku" => json_str(d, "oldSku"),
                    "oldVendorCode" => json_str(d, "oldVendorCode"),
                    "newShkId" => json_num(d, "newShkId"),
                    "newColor" => json_str(d, "newColor"),
                    "newSize" => json_str(d, "newSize"),
                    "newSku" => json_str(d, "newSku"),
                    "newVendorCode" => json_str(d, "newVendorCode"),
                    "photoUrls" => photos.clone(),
                    _ => String::new(),
                })
                .collect(),
        );
    }
}

pub fn total_count(root: &Value) -> Option<i64> {
    root.get("data")
        .and_then(|d| d.get("total"))
        .and_then(|t| t.as_i64())
}

fn fmt_ts(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    sheet_dates::format_wb_timestamp(t)
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
