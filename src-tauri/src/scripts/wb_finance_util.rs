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

//! Общие helpers finance-api (отчёт реализации / список отчётов).

use crate::core::sheet_dates;
use serde_json::Value;

pub fn resolve_period(ctx_period: Option<&str>) -> &'static str {
    match ctx_period.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("daily") => "daily",
        _ => "weekly",
    }
}

pub fn json_cell(name: &str, cabinet: &str, row: &Value) -> String {
    if name == "cabinet" {
        return cabinet.to_string();
    }
    if name == "reportTypeName" {
        return report_type_name(row);
    }
    let v = row.get(name).or_else(|| {
        // camelCase aliases used by WB
        None
    });
    match v {
        None => String::new(),
        Some(Value::Null) => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => {
            if looks_like_date_field(name) {
                sheet_dates::format_wb_timestamp(s)
            } else {
                s.clone()
            }
        }
        Some(other) => other.to_string(),
    }
}

fn looks_like_date_field(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("date") || n.ends_with("dt") || n == "createdate"
}

fn report_type_name(row: &Value) -> String {
    match row.get("reportType").and_then(|v| v.as_i64()).or_else(|| {
        row.get("reportType")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }) {
        Some(1) => "Основной".into(),
        Some(2) => "Дополнительный".into(),
        Some(n) => n.to_string(),
        None => row
            .get("reportType")
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default(),
    }
}

pub fn build_data_rows(
    cabinet: &str,
    items: &[Value],
    field_names: &[&str],
) -> Vec<Vec<String>> {
    items
        .iter()
        .map(|it| {
            field_names
                .iter()
                .map(|n| json_cell(n, cabinet, it))
                .collect()
        })
        .collect()
}
