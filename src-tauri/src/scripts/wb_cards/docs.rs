//! ТН-ВЭД и сертификаты/декларации из карточки WB (паритет CardsScript.FieldMapping).

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
        let cid = ch
            .get("id")
            .and_then(|v| v.as_i64())
            .or_else(|| ch.get("id").and_then(|v| v.as_u64()).map(|u| u as i64));
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
}
