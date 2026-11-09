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

//! Поля `wb_adv_norm_query_stats` — schema `POST /adv/v1/normquery/stats`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("advertId", true),
        field("nmId", true),
        field("date", true),
        field("normQuery", true),
        field("views", true),
        field("clicks", true),
        field("atbs", true),
        field("orders", true),
        field("ctr", true),
        field("cpc", true),
        field("cpm", true),
        field("avgPos", true),
        field("shks", true),
        field("spend", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_norm_query_stats.{name}")),
        selected,
    }
}

pub fn append_stats_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let items = root
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in &items {
        let advert_id = json_num(item, "advertId");
        let nm_id = json_num(item, "nmId");
        let daily = item
            .get("dailyStats")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        for day in &daily {
            let date = fmt_date(json_str(day, "date"));
            for st in day_stats(day) {
                out.push(row(
                    &selected,
                    cabinet,
                    &advert_id,
                    &nm_id,
                    &date,
                    st,
                ));
            }
        }
    }
}

/// `stat` — объект или массив объектов schema.
fn day_stats(day: &Value) -> Vec<&Value> {
    match day.get("stat") {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(obj) if obj.is_object() => vec![obj],
        _ => Vec::new(),
    }
}

fn row(
    selected: &[&ScriptField],
    cabinet: &str,
    advert_id: &str,
    nm_id: &str,
    date: &str,
    m: &Value,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "advertId" => advert_id.to_string(),
            "nmId" => nm_id.to_string(),
            "date" => date.to_string(),
            "normQuery" => json_str(m, "normQuery"),
            "views" => json_num(m, "views"),
            "clicks" => json_num(m, "clicks"),
            "atbs" => json_num(m, "atbs"),
            "orders" => json_num(m, "orders"),
            "ctr" => json_num(m, "ctr"),
            "cpc" => json_num(m, "cpc"),
            "cpm" => json_num(m, "cpm"),
            "avgPos" => json_num(m, "avgPos"),
            "shks" => json_num(m, "shks"),
            "spend" => json_num(m, "spend"),
            _ => String::new(),
        })
        .collect()
}

fn fmt_date(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return sheet_dates::format_date(d);
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
