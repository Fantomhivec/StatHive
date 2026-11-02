//! Поля и строки для `wb_adv_upd` (лист «Расходы»).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime};
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("updNum", true),
        field("updTime", true),
        field("updSum", true),
        field("advertId", true),
        field("campName", true),
        field("advertType", true),
        field("paymentType", true),
        field("advertStatus", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_upd.{name}")),
        selected,
    }
}

pub fn map_advert_status(status: Option<i64>) -> String {
    match status {
        Some(-1) => "Удалена".into(),
        Some(4) => "Готова к запуску".into(),
        Some(7) => "Завершена".into(),
        Some(8) => "Отменена".into(),
        Some(9) => "Активна".into(),
        Some(11) => "На паузе".into(),
        Some(s) => s.to_string(),
        None => String::new(),
    }
}

pub fn build_chunks(from: NaiveDate, to: NaiveDate, max_days: i64) -> Vec<(NaiveDate, NaiveDate)> {
    let mut out = Vec::new();
    let mut cursor = from;
    while cursor <= to {
        let end = (cursor + Duration::days(max_days - 1)).min(to);
        out.push((cursor, end));
        cursor = end + Duration::days(1);
    }
    out
}

pub fn parse_upd_time(raw: &str) -> Option<NaiveDateTime> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    const DT: &[&str] = &[
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%SZ",
    ];
    for fmt in DT {
        if let Ok(dt) = NaiveDateTime::parse_from_str(t, fmt) {
            return Some(dt);
        }
    }
    const D: &[&str] = &["%d.%m.%Y", "%Y-%m-%d"];
    for fmt in D {
        if let Ok(d) = NaiveDate::parse_from_str(t, fmt) {
            return d.and_hms_opt(0, 0, 0);
        }
    }
    DateTime::parse_from_rfc3339(t)
        .ok()
        .map(|dto| dto.naive_local())
}

pub fn format_upd_time(raw: &str) -> String {
    if let Some(dt) = parse_upd_time(raw) {
        sheet_dates::format_datetime(dt)
    } else {
        raw.trim().to_string()
    }
}

pub fn row_from_item(cabinet: &str, item: &Value, fields: &[ScriptField]) -> Vec<String> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let upd_time_raw = json_str(item, "updTime");
    let status = map_advert_status(json_i64(item, "advertStatus"));
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "updNum" => json_num_str(item, "updNum"),
            "updTime" => format_upd_time(&upd_time_raw),
            "updSum" => json_num_str(item, "updSum"),
            "advertId" => json_num_str(item, "advertId"),
            "campName" => json_str(item, "campName"),
            "advertType" => json_num_str(item, "advertType"),
            "paymentType" => json_str(item, "paymentType"),
            "advertStatus" => status.clone(),
            _ => String::new(),
        })
        .collect()
}

pub fn upd_time_sort_key(raw: &str) -> NaiveDate {
    parse_upd_time(raw)
        .map(|dt| dt.date())
        .unwrap_or(NaiveDate::MAX)
}

fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn json_num_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn json_i64(v: &Value, key: &str) -> Option<i64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}
