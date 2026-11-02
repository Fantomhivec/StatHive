//! HTTP helpers для `/api/v2/nm-report/downloads`.

use crate::core::script_field::ScriptField;
use crate::core::rate_limit::RateLimitProfile;
use crate::core::request_sender::{Auth, Limit, RequestSender};
use crate::core::sheet_dates;
use chrono::NaiveDate;
use serde_json::{json, Value};
use std::io::{Cursor, Read};

const BASE: &str = "https://seller-analytics-api.wildberries.ru/api/v2/nm-report/downloads";

pub async fn create_detail_history(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(), String> {
    let body = json!({
        "id": id,
        "reportType": "DETAIL_HISTORY_REPORT",
        "userReportName": format!("UR {}..{}", from, to),
        "params": {
            "nmIDs": [],
            "subjectIds": [],
            "brandNames": [],
            "tagIds": [],
            "startDate": from.format("%Y-%m-%d").to_string(),
            "endDate": to.format("%Y-%m-%d").to_string(),
            "timezone": "Europe/Moscow",
            "aggregationLevel": "day",
            "skipDeletedNm": false,
        }
    });
    request
        .post_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            BASE,
            Auth::Raw(token),
            &body,
        )
        .await?;
    Ok(())
}

pub async fn create_stock_history_daily(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(), String> {
    let body = json!({
        "id": id,
        "reportType": "STOCK_HISTORY_DAILY_CSV",
        "userReportName": format!("UR stock {}..{}", from, to),
        "params": {
            "nmIds": [],
            "subjectIds": [],
            "brandNames": [],
            "tagIds": [],
            "currentPeriod": {
                "start": from.format("%Y-%m-%d").to_string(),
                "end": to.format("%Y-%m-%d").to_string(),
            },
            "stockType": "ALL",
            "skipDeletedNm": false,
        }
    });
    request
        .post_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            BASE,
            Auth::Raw(token),
            &body,
        )
        .await?;
    Ok(())
}

pub async fn list_downloads(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    download_ids: &[String],
) -> Result<Vec<Value>, String> {
    let url = if download_ids.is_empty() {
        BASE.to_string()
    } else {
        let q: Vec<String> = download_ids
            .iter()
            .map(|id| format!("filter[downloadIds]={}", urlencoding::encode(id)))
            .collect();
        format!("{BASE}?{}", q.join("&"))
    };
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
    Ok(extract_list(&v))
}

pub async fn download_file_rows(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    download_id: &str,
) -> Result<Vec<Vec<String>>, String> {
    let url = format!("{BASE}/file/{download_id}");
    let bytes = request
        .download_bytes_auth(
            Limit {
                cabinet,
                scope,
                profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await?;
    parse_report_bytes(&bytes)
}

fn extract_list(v: &Value) -> Vec<Value> {
    if let Some(a) = v.as_array() {
        return a.clone();
    }
    if let Some(a) = v.pointer("/data").and_then(|x| x.as_array()) {
        return a.clone();
    }
    if let Some(a) = v.pointer("/data/reports").and_then(|x| x.as_array()) {
        return a.clone();
    }
    Vec::new()
}

pub fn is_success(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "success" | "done" | "completed" | "ready"
    )
}

pub fn is_failed(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "failed" | "error" | "canceled" | "cancelled"
    )
}

pub fn item_status(it: &Value) -> String {
    it.get("status")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn item_id(it: &Value) -> String {
    it.get("id")
        .or_else(|| it.get("downloadId"))
        .map(|x| match x {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

pub fn create_row(cabinet: &str, from: NaiveDate, to: NaiveDate, id: &str, report_type: &str) -> Vec<String> {
    vec![
        cabinet.to_string(),
        id.to_string(),
        sheet_dates::format_date(from),
        sheet_dates::format_date(to),
        report_type.to_string(),
    ]
}

pub fn list_cell(name: &str, cabinet: &str, it: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "downloadId" => item_id(it),
        "status" => item_status(it),
        "name" => str_any(it, "name").or_else(|| str_any(it, "userReportName")).unwrap_or_default(),
        "createdAt" => {
            let raw = str_any(it, "createdAt").unwrap_or_default();
            if raw.is_empty() {
                raw
            } else {
                sheet_dates::format_wb_timestamp(&raw)
            }
        }
        "size" => any(it, "size"),
        "startDate" => period_edge(it, true),
        "endDate" => period_edge(it, false),
        _ => any(it, name),
    }
}

fn period_edge(it: &Value, start: bool) -> String {
    let key = if start { "startDate" } else { "endDate" };
    if let Some(s) = str_any(it, key) {
        return sheet_dates::format_wb_timestamp(&s);
    }
    let path = if start {
        "/params/startDate"
    } else {
        "/params/endDate"
    };
    it.pointer(path)
        .and_then(|x| x.as_str())
        .map(sheet_dates::format_wb_timestamp)
        .unwrap_or_default()
}

fn str_any(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn any(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Null) | None => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

// --- parse (merged) ---
pub fn parse_report_bytes(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    if bytes.len() >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4B {
        return parse_zip_csv(bytes);
    }
    parse_csv(bytes)
}

fn parse_zip_csv(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("ZIP: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("ZIP entry: {e}"))?;
        let name = file.name().to_ascii_lowercase();
        if !(name.ends_with(".csv") || name.ends_with(".tsv") || name.ends_with(".txt")) {
            continue;
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("ZIP read: {e}"))?;
        return parse_csv(&buf);
    }
    Err("В ZIP нет CSV".into())
}

fn parse_csv(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let text = if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let sep = if line.contains(';') {
            ';'
        } else if line.contains('\t') {
            '\t'
        } else {
            ','
        };
        rows.push(
            line.split(sep)
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect(),
        );
    }
    Ok(rows)
}

// --- fields (merged) ---
pub fn create_fields() -> Vec<ScriptField> {
    vec![
        f("cabinet", true),
        f("downloadId", true),
        f("dateFrom", true),
        f("dateTo", true),
        f("reportType", true),
    ]
}

pub fn list_fields() -> Vec<ScriptField> {
    vec![
        f("cabinet", true),
        f("downloadId", true),
        f("status", true),
        f("name", false),
        f("createdAt", true),
        f("size", false),
        f("startDate", false),
        f("endDate", false),
    ]
}

/// CSV-выгрузка: кабинет + колонки файла (динамически).
pub fn download_fields() -> Vec<ScriptField> {
    vec![f("cabinet", true), f("csv", true)]
}

fn f(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.nm_report.{name}")),
        selected,
    }
}
