//! Поля и ячейки `claims_list` (паритет ClaimsScript).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

const SPECS: &[(&str, bool)] = &[
    ("cabinet", true),
    ("id", true),
    ("claim_type", true),
    ("status", true),
    ("status_ex", true),
    ("nm_id", true),
    ("user_comment", true),
    ("wb_comment", true),
    ("dt", true),
    ("imt_name", true),
    ("order_dt", true),
    ("dt_update", true),
    ("photos", false),
    ("videos", false),
    ("actions", false),
    ("price", true),
    ("currency_code", true),
    ("srid", true),
];

pub fn default_fields() -> Vec<ScriptField> {
    SPECS
        .iter()
        .map(|(name, selected)| ScriptField {
            name: (*name).into(),
            title: crate::core::i18n::t(&format!("fields.claims_list.{name}")),
            selected: *selected,
        })
        .collect()
}

pub fn is_media(name: &str) -> bool {
    name == "photos" || name == "videos"
}

pub fn cell(name: &str, cabinet: &str, claim: &Value) -> String {
    if name == "cabinet" {
        return cabinet.to_string();
    }
    match name {
        "id" => text(claim.get("id")),
        "claim_type" => claim_type(num(claim, "claim_type")),
        "status" => status(num(claim, "status")),
        "status_ex" => status_ex(num(claim, "status_ex")),
        "nm_id" => num_str(claim, "nm_id"),
        "user_comment" => text(claim.get("user_comment")),
        "wb_comment" => text(claim.get("wb_comment")),
        "dt" => stamp(claim, "dt"),
        "imt_name" => text(claim.get("imt_name")),
        "order_dt" => stamp(claim, "order_dt"),
        "dt_update" => stamp(claim, "dt_update"),
        "actions" => join_actions(claim),
        "price" => price(claim.get("price")),
        "currency_code" => text(claim.get("currency_code")),
        "srid" => text(claim.get("srid")),
        _ => String::new(),
    }
}

pub fn photos(claim: &Value) -> Vec<String> {
    urls(claim, "photos")
}

pub fn videos(claim: &Value) -> Vec<String> {
    urls(claim, "video_paths")
}

fn urls(claim: &Value, key: &str) -> Vec<String> {
    claim
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .map(abs_url)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn abs_url(raw: &str) -> String {
    let t = raw.trim();
    if t.starts_with("//") {
        format!("https:{t}")
    } else {
        t.to_string()
    }
}

fn stamp(claim: &Value, key: &str) -> String {
    let raw = text(claim.get(key));
    if raw.is_empty() {
        String::new()
    } else {
        sheet_dates::format_wb_timestamp(&raw)
    }
}

fn text(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

fn num(claim: &Value, key: &str) -> i64 {
    claim.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn num_str(claim: &Value, key: &str) -> String {
    match claim.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn price(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n
            .as_f64()
            .map(|f| format!("{f:.2}"))
            .unwrap_or_else(|| n.to_string()),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn join_actions(claim: &Value) -> String {
    claim
        .get("actions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| match x {
                    Value::String(s) if !s.is_empty() => Some(s.clone()),
                    Value::Null => None,
                    other => {
                        let s = other.to_string();
                        if s.is_empty() || s == "null" {
                            None
                        } else {
                            Some(s)
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default()
}

fn claim_type(n: i64) -> String {
    match n {
        1 => "Портал покупателей".into(),
        3 => "Чат".into(),
        other => other.to_string(),
    }
}

fn status(n: i64) -> String {
    match n {
        0 => "На рассмотрении".into(),
        1 => "Отказ".into(),
        2 => "Одобрено".into(),
        other => other.to_string(),
    }
}

fn status_ex(n: i64) -> String {
    match n {
        0 => "Заявка на рассмотрении".into(),
        1 => "Товар остается у покупателя (Заявка отклонена)".into(),
        2 => "Покупатель сдает товар на WB, товар отправляется в утиль".into(),
        5 => "Товар остается у покупателя (Заявка одобрена)".into(),
        8 => "Товар будет возвращён в реализацию после проверки WB".into(),
        10 => "Товар возвращается продавцу".into(),
        other => other.to_string(),
    }
}
