//! Paid storage HTTP helpers (seller-analytics-api).

use crate::core::script_field::ScriptField;
use crate::core::rate_limit::RateLimitProfile;
use crate::core::request_sender::{Auth, Limit, RequestSender};
use crate::core::sheet_dates;
use chrono::{Duration, NaiveDate};
use serde_json::Value;

const BASE: &str = "https://seller-analytics-api.wildberries.ru/api/v1/paid_storage";

pub fn split_intervals(from: NaiveDate, to: NaiveDate, max_days: i64) -> Vec<(NaiveDate, NaiveDate)> {
    let mut out = Vec::new();
    let mut cur = from;
    while cur <= to {
        let end = (cur + Duration::days(max_days - 1)).min(to);
        out.push((cur, end));
        cur = end + Duration::days(1);
    }
    out
}

pub async fn create_task(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<String, String> {
    let url = format!(
        "{BASE}?dateFrom={}&dateTo={}",
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );
    let v = request
        .get_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await?;
    v.pointer("/data/taskId")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Кабинет {cabinet}: пустой taskId"))
}

pub async fn task_status(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    task_id: &str,
) -> Result<String, String> {
    let url = format!("{BASE}/tasks/{task_id}/status");
    let v = request
        .get_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await?;
    Ok(v
        .pointer("/data/status")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string())
}

pub async fn download_report(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    task_id: &str,
) -> Result<Vec<Value>, String> {
    let url = format!("{BASE}/tasks/{task_id}/download");
    let v = request
        .get_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await?;
    Ok(match v {
        Value::Array(a) => a,
        other => other.as_array().cloned().unwrap_or_default(),
    })
}

pub fn is_done_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "done" | "success" | "completed"
    )
}

pub fn is_failed_status(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "failed" | "error" | "canceled" | "cancelled"
    )
}

pub fn report_cell(name: &str, cabinet: &str, task_id: &str, it: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "taskId" => task_id.to_string(),
        "date" | "originalDate" | "tariffFixDate" | "tariffLowerDate" => {
            let raw = json_str(it, name);
            if raw.is_empty() {
                raw
            } else {
                sheet_dates::format_wb_timestamp(&raw)
            }
        }
        "nmId" => json_any(it, "nmId").or_else(|| json_any(it, "nmID")).unwrap_or_default(),
        _ => json_any(it, name).unwrap_or_default(),
    }
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_any(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::Null => None,
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

pub fn create_row(cabinet: &str, from: NaiveDate, to: NaiveDate, task_id: &str) -> Vec<String> {
    vec![
        cabinet.to_string(),
        sheet_dates::format_date(from),
        sheet_dates::format_date(to),
        task_id.to_string(),
    ]
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("taskId", true),
        field("date", true),
        field("logWarehouseCoef", true),
        field("officeId", true),
        field("warehouse", true),
        field("warehouseCoef", true),
        field("giId", true),
        field("chrtId", true),
        field("size", true),
        field("barcode", true),
        field("subject", true),
        field("brand", true),
        field("vendorCode", true),
        field("nmId", true),
        field("volume", true),
        field("calcType", true),
        field("warehousePrice", true),
        field("barcodesCount", true),
        field("palletPlaceCode", true),
        field("palletCount", true),
        field("originalDate", true),
        field("loyaltyDiscount", true),
        field("tariffFixDate", true),
        field("tariffLowerDate", true),
    ]
}

pub fn create_task_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("dateFrom", true),
        field("dateTo", true),
        field("taskId", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.paid_storage.{name}")),
        selected,
    }
}
