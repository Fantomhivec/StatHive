//! Warehouse remains HTTP (seller-analytics-api).

use crate::core::rate_limit::RateLimitProfile;
use crate::core::request_sender::{Auth, Limit, RequestSender};
use crate::core::script_field::ScriptField;
use serde_json::Value;

pub const BASE: &str = "https://seller-analytics-api.wildberries.ru/api/v1/warehouse_remains";

#[derive(Clone, Copy)]
pub struct GroupBy {
    pub nm: bool,
    pub size: bool,
    pub brand: bool,
    pub subject: bool,
    pub sa: bool,
    pub barcode: bool,
}

impl Default for GroupBy {
    fn default() -> Self {
        Self {
            nm: true,
            size: true,
            brand: false,
            subject: false,
            sa: false,
            barcode: false,
        }
    }
}

pub async fn create_task(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    g: GroupBy,
) -> Result<String, String> {
    let url = format!(
        "{BASE}?locale=ru&groupByNm={}&groupBySize={}&groupByBrand={}\
         &groupBySubject={}&groupBySa={}&groupByBarcode={}",
        g.nm, g.size, g.brand, g.subject, g.sa, g.barcode
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
        other => other
            .pointer("/data")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default(),
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

/// Flatten nested `warehouses[]` into one object per warehouse row.
pub fn flatten_report_items(items: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for it in items {
        let Some(warehouses) = it.get("warehouses").and_then(|x| x.as_array()) else {
            out.push(it.clone());
            continue;
        };
        if warehouses.is_empty() {
            out.push(it.clone());
            continue;
        }
        for wh in warehouses {
            let mut row = it.clone();
            if let Some(obj) = row.as_object_mut() {
                obj.remove("warehouses");
                if let Some(w) = wh.as_object() {
                    for (k, v) in w {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            out.push(row);
        }
    }
    out
}

pub fn report_cell(name: &str, cabinet: &str, task_id: &str, it: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "taskId" => task_id.to_string(),
        "nmId" => json_any(it, "nmId")
            .or_else(|| json_any(it, "nmID"))
            .unwrap_or_default(),
        "vendorCode" => json_any(it, "vendorCode")
            .or_else(|| json_any(it, "saName"))
            .unwrap_or_default(),
        "subjectName" => json_any(it, "subjectName")
            .or_else(|| json_any(it, "subject"))
            .unwrap_or_default(),
        "brand" => json_any(it, "brand")
            .or_else(|| json_any(it, "brandName"))
            .unwrap_or_default(),
        "techSize" => json_any(it, "techSize")
            .or_else(|| json_any(it, "size"))
            .unwrap_or_default(),
        _ => json_any(it, name).unwrap_or_default(),
    }
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

pub fn create_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("taskId", true),
        field("createdAt", true),
    ]
}

pub fn download_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("taskId", true),
        field("brand", true),
        field("subjectName", true),
        field("vendorCode", true),
        field("nmId", true),
        field("barcode", true),
        field("techSize", true),
        field("volume", true),
        field("warehouseName", true),
        field("quantity", true),
        field("inWayToClient", true),
        field("inWayFromClient", true),
        field("quantityFull", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.warehouse_remains.{name}")),
        selected,
    }
}
