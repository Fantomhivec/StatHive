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

//! ТН-ВЭД / cert+decl helpers + join всех documents.items по текущей schema cards/list.

use crate::core::sheet_dates;
use serde_json::Value;

const CERT_DOCUMENT_TYPE: i64 = 1;
const DECL_DOCUMENT_TYPE: i64 = 2;
/// Два ID характеристики ТН-ВЭД на карточке WB → в ячейку как x/y.
const TNVED_CHARC_ID_A: i64 = 15_000_001;
const TNVED_CHARC_ID_B: i64 = 15_004_139;

pub fn tnved_pair(card: &Value) -> String {
    let Some(arr) = card.get("characteristics").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let x = charc_value_by_id(arr, TNVED_CHARC_ID_A);
    let y = charc_value_by_id(arr, TNVED_CHARC_ID_B);
    if x.is_empty() && y.is_empty() {
        return String::new();
    }
    format!("{x}/{y}")
}

/// Convenience: ставка НДС из characteristics по имени (не поле schema API).
pub fn vat_rate_from_charcs(card: &Value) -> String {
    let Some(arr) = card.get("characteristics").and_then(|c| c.as_array()) else {
        return String::new();
    };
    for ch in arr {
        let name = ch
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.contains("ндс") || name.contains("vat") {
            let v = normalize_charc_value(ch.get("value")).trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
    }
    String::new()
}

pub fn document_numbers(card: &Value) -> String {
    main_documents(card)
        .filter_map(|d| {
            let n = json_trim(d, "number");
            if n.is_empty() {
                None
            } else {
                Some(n)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn document_start_dates(card: &Value) -> String {
    main_documents(card)
        .map(|d| sheet_dates::format_wb_timestamp(&json_trim(d, "startDate")))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn document_end_dates(card: &Value) -> String {
    let dates: Vec<String> = main_documents(card)
        .map(|d| {
            if d.get("isEndless").and_then(|v| v.as_bool()).unwrap_or(false) {
                String::new()
            } else {
                sheet_dates::format_wb_timestamp(&json_trim(d, "endDate"))
            }
        })
        .collect();
    if dates.is_empty() || dates.iter().all(|s| s.is_empty()) {
        return String::new();
    }
    dates.join("\n")
}

pub fn document_check_statuses(card: &Value) -> String {
    let statuses: Vec<String> = main_documents(card)
        .map(format_document_check_status)
        .collect();
    if statuses.is_empty() || statuses.iter().all(|s| s.trim().is_empty()) {
        return String::new();
    }
    statuses.join("\n")
}

pub fn exclude_documents(card: &Value) -> String {
    path_leaf(card, &["documents", "excludeDocuments"])
}

pub fn overall_verdict_field(card: &Value, key: &str) -> String {
    let raw = path_leaf(card, &["documents", "overallVerdict", key]);
    if key == "createdAt" {
        sheet_dates::format_wb_timestamp(&raw)
    } else {
        raw
    }
}

/// Join `documents.items[*].key` (or nested verdict.*) with newlines; dates formatted.
pub fn items_field_join(card: &Value, key: &str) -> String {
    let Some(items) = card
        .get("documents")
        .and_then(|d| d.get("items"))
        .and_then(|i| i.as_array())
    else {
        return String::new();
    };
    let rows: Vec<String> = items
        .iter()
        .map(|d| match key {
            "startDate" | "endDate" | "createdAt" => {
                sheet_dates::format_wb_timestamp(&json_trim(d, key))
            }
            "verdict.verified" => path_leaf(d, &["verdict", "verified"]),
            "verdict.status" => path_leaf(d, &["verdict", "status"]),
            "verdict.reason" => path_leaf(d, &["verdict", "reason"]),
            "verdict.createdAt" => {
                sheet_dates::format_wb_timestamp(&path_leaf(d, &["verdict", "createdAt"]))
            }
            "verdict.additionalData" => additional_data_cell(d.get("verdict").and_then(|v| v.get("additionalData"))),
            other => json_trim(d, other),
        })
        .collect();
    if rows.iter().all(|s| s.is_empty()) {
        String::new()
    } else {
        rows.join("\n")
    }
}

fn additional_data_cell(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn main_documents(card: &Value) -> impl Iterator<Item = &Value> {
    card.get("documents")
        .and_then(|d| d.get("items"))
        .and_then(|i| i.as_array())
        .into_iter()
        .flatten()
        .filter(|d| {
            let t = d.get("type").and_then(|v| v.as_i64()).unwrap_or(0);
            t == CERT_DOCUMENT_TYPE || t == DECL_DOCUMENT_TYPE
        })
}

fn format_document_check_status(d: &Value) -> String {
    let Some(verdict) = d.get("verdict") else {
        return "Не пройдена".into();
    };
    if verdict
        .get("verified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return "Проверка пройдена".into();
    }
    let reason = verdict
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !reason.is_empty() {
        return reason.to_string();
    }
    "Не пройдена".into()
}

fn charc_value_by_id(charcs: &[Value], id: i64) -> String {
    for ch in charcs {
        let cid = ch.get("id").and_then(|v| v.as_i64());
        if cid == Some(id) {
            return normalize_charc_value(ch.get("value")).trim().to_string();
        }
    }
    String::new()
}

fn normalize_charc_value(value: Option<&Value>) -> String {
    let Some(v) = value else {
        return String::new();
    };
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .filter_map(|x| match x {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Null => None,
                other => {
                    let s = other.to_string().trim_matches('"').to_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string().trim_matches('"').to_string(),
    }
}

fn path_leaf(root: &Value, parts: &[&str]) -> String {
    let mut cur = root;
    for (i, p) in parts.iter().enumerate() {
        match cur.get(*p) {
            Some(next) if i + 1 == parts.len() => {
                return match next {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Null => String::new(),
                    other => other.to_string().trim_matches('"').to_string(),
                };
            }
            Some(next) => cur = next,
            None => return String::new(),
        }
    }
    String::new()
}

fn json_trim(v: &Value, key: &str) -> String {
    v.get(key)
        .map(|x| match x {
            Value::String(s) => s.trim().to_string(),
            Value::Null => String::new(),
            other => other.to_string().trim_matches('"').trim().to_string(),
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tnved_from_charc_ids() {
        let card = json!({
            "characteristics": [
                { "id": 15000001, "name": "ТНВЭД", "value": ["850440"] },
                { "id": 15004139, "name": "ТНВЭД2", "value": "90" }
            ]
        });
        assert_eq!(tnved_pair(&card), "850440/90");
    }

    #[test]
    fn doc_number_and_status_from_documents() {
        let card = json!({
            "documents": {
                "items": [
                    {
                        "type": 1,
                        "number": "ЕАЭС N RU Д-RU.РА01.В.12345",
                        "startDate": "2024-01-15T00:00:00Z",
                        "endDate": "2029-01-15T00:00:00Z",
                        "isEndless": false,
                        "verdict": { "verified": true }
                    },
                    { "type": 99, "number": "ignored" }
                ]
            }
        });
        assert!(document_numbers(&card).contains("ЕАЭС"));
        assert_eq!(document_check_statuses(&card), "Проверка пройдена");
        assert!(!document_start_dates(&card).is_empty());
    }

    #[test]
    fn overall_verdict_and_items_join_from_example_shape() {
        let card = json!({
            "documents": {
                "excludeDocuments": false,
                "overallVerdict": {
                    "isFullyChecked": true,
                    "status": 1,
                    "reason": null,
                    "createdAt": "2026-05-28T18:19:19.99444+03:00"
                },
                "items": [
                    {
                        "id": "019f3c09-8c1f-7063-aee4-c45969fb2dc5",
                        "type": 1,
                        "number": "123456",
                        "productNumber": "123456",
                        "tradeName": "Trade Name",
                        "applicant": "Applicant Name",
                        "startDate": "2023-01-15T03:00:00+03:00",
                        "endDate": "2026-01-14T03:00:00+03:00",
                        "verdict": {
                            "verified": true,
                            "status": 1,
                            "reason": null,
                            "additionalData": null,
                            "createdAt": "2026-05-28T18:10:43.375306+03:00"
                        },
                        "createdAt": "2026-05-28T18:09:25.394692+03:00"
                    }
                ]
            }
        });
        assert_eq!(exclude_documents(&card), "false");
        assert_eq!(overall_verdict_field(&card, "isFullyChecked"), "true");
        assert_eq!(overall_verdict_field(&card, "status"), "1");
        assert!(overall_verdict_field(&card, "reason").is_empty());
        assert_eq!(items_field_join(&card, "number"), "123456");
        assert_eq!(items_field_join(&card, "verdict.verified"), "true");
        assert!(items_field_join(&card, "verdict.additionalData").is_empty());
    }
}
