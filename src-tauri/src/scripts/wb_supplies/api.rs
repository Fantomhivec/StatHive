//! WB supplies HTTP helpers (marketplace FBS + supplies-api FBO).
//! FBO list: official POST /api/v1/supplies.

use crate::core::rate_limit::RateLimitProfile;
use crate::core::request_sender::{Auth, Limit, RequestSender};
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use chrono::NaiveDate;
use serde_json::{json, Value};

pub const FBS_BASE: &str = "https://marketplace-api.wildberries.ru/api/v3/supplies";
pub const FBO_BASE: &str = "https://supplies-api.wildberries.ru/api/v1/supplies";
pub const FBO_WH: &str = "https://supplies-api.wildberries.ru/api/v1/warehouses";

const FBO_PAGE: i64 = 1000;

pub async fn list_fbs(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut next: i64 = 0;
    loop {
        let url = format!("{FBS_BASE}?limit=1000&next={next}");
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
        let supplies = v
            .get("supplies")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let n = supplies.len();
        out.extend(supplies);
        next = v.get("next").and_then(|x| x.as_i64()).unwrap_or(0);
        if n == 0 || next == 0 {
            break;
        }
    }
    Ok(out)
}

/// Official API: POST supplies-api `/api/v1/supplies` with dates + statusIDs.
pub async fn list_fbo(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Value>, String> {
    let body = json!({
        "dates": [{
            "from": from.format("%Y-%m-%d").to_string(),
            "till": to.format("%Y-%m-%d").to_string(),
            "type": "factDate"
        }],
        "statusIDs": [1, 2, 3, 4, 5, 6]
    });
    let mut out = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let url = format!("{FBO_BASE}?limit={FBO_PAGE}&offset={offset}");
        let v = request
            .post_json(
                Limit {
                    cabinet,
                    scope,
                    profile,
                },
                &url,
                Auth::Raw(token),
                &body,
            )
            .await?;
        let page = match v {
            Value::Array(a) => a,
            other => other.as_array().cloned().unwrap_or_default(),
        };
        let n = page.len() as i64;
        out.extend(page);
        if n < FBO_PAGE {
            break;
        }
        offset += FBO_PAGE;
    }
    Ok(out)
}

pub async fn supply_info(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    supply_id: &str,
) -> Result<Value, String> {
    let url = format!("{FBS_BASE}/{supply_id}");
    request
        .get_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await
}

/// GET supplies-api `/api/v1/supplies/{ID}/goods` with limit/offset.
pub async fn supply_goods(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    supply_id: &str,
) -> Result<Vec<Value>, String> {
    let enc = urlencoding_simple(supply_id.trim());
    let mut out = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let url = format!(
            "{FBO_BASE}/{enc}/goods?limit={FBO_PAGE}&offset={offset}&isPreorderID=false"
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
        let page = match v {
            Value::Array(a) => a,
            other => other
                .get("goods")
                .and_then(|x| x.as_array())
                .cloned()
                .unwrap_or_default(),
        };
        let n = page.len() as i64;
        out.extend(page);
        if n < FBO_PAGE {
            break;
        }
        offset += FBO_PAGE;
    }
    Ok(out)
}

pub async fn fbo_warehouses(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
) -> Result<Vec<Value>, String> {
    let v = request
        .get_json(
            Limit {
                cabinet,
                scope,
                profile,
            },
            FBO_WH,
            Auth::Raw(token),
        )
        .await?;
    Ok(match v {
        Value::Array(a) => a,
        other => other.as_array().cloned().unwrap_or_default(),
    })
}

fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn json_cell(it: &Value, name: &str) -> String {
    match it.get(name) {
        Some(Value::Null) | None => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => {
            if name.to_ascii_lowercase().contains("date") || name.ends_with("At") {
                let t = s.trim();
                if t.len() >= 10 {
                    return sheet_dates::format_wb_timestamp(t);
                }
            }
            s.clone()
        }
        Some(other) => other.to_string(),
    }
}

fn first_cell(it: &Value, keys: &[&str]) -> String {
    for k in keys {
        let v = json_cell(it, k);
        if !v.is_empty() {
            return v;
        }
    }
    String::new()
}

fn status_id_name(id: &str) -> String {
    match id.trim() {
        "1" => "Не запланировано".into(),
        "2" => "Запланировано".into(),
        "3" => "Отгрузка разрешена".into(),
        "4" => "Идёт приёмка".into(),
        "5" => "Принято".into(),
        "6" => "Отгружено на воротах".into(),
        _ => id.to_string(),
    }
}

fn box_type_id_name(id: &str) -> String {
    match id.trim() {
        "0" => "Без коробов".into(),
        "1" | "2" => "Короба".into(),
        "5" => "Монопаллеты".into(),
        "6" => "Суперсейф".into(),
        _ => id.to_string(),
    }
}

pub fn field(prefix: &str, name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.{prefix}.{name}")),
        selected,
    }
}

pub fn list_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "id",
        "name",
        "done",
        "closedAt",
        "scanDt",
        "createdAt",
        "cargoType",
    ]
    .into_iter()
    .map(|n| field("supplies", n, true))
    .collect()
}

pub fn fbo_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "phone",
        "supplyID",
        "preorderID",
        "createDate",
        "supplyDate",
        "factDate",
        "updatedDate",
        "statusID",
        "statusIDName",
        "boxTypeID",
        "boxTypeIDName",
        "isBoxOnPallet",
    ]
    .into_iter()
    .map(|n| field("supplies_fbo", n, true))
    .collect()
}

pub fn info_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "id",
        "name",
        "done",
        "closedAt",
        "scanDt",
        "createdAt",
        "cargoType",
    ]
    .into_iter()
    .map(|n| field("supply_info", n, true))
    .collect()
}

pub fn goods_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "supplyID",
        "barcode",
        "vendorCode",
        "nmID",
        "needKiz",
        "tnved",
        "techSize",
        "color",
        "supplierBoxAmount",
        "quantity",
        "readyForSaleQuantity",
        "acceptedQuantity",
        "unloadingQuantity",
    ]
    .into_iter()
    .map(|n| field("supplies_goods", n, true))
    .collect()
}

pub fn warehouses_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "id",
        "name",
        "address",
        "workTime",
        "isActive",
        "isTransitActive",
    ]
    .into_iter()
    .map(|n| field("supplies_warehouses", n, true))
    .collect()
}

pub fn row_from(
    cabinet: &str,
    it: &Value,
    selected: &[&ScriptField],
    extra: &[(&str, &str)],
) -> Vec<String> {
    selected
        .iter()
        .map(|f| {
            if f.name == "cabinet" {
                return cabinet.to_string();
            }
            for (k, v) in extra {
                if f.name == *k {
                    return (*v).to_string();
                }
            }
            match f.name.as_str() {
                "supplyID" | "supplyId" => {
                    first_cell(it, &["supplyID", "supplyId", "id", "ID"])
                }
                "preorderID" | "preorderId" => first_cell(it, &["preorderID", "preorderId"]),
                "statusID" | "statusId" => first_cell(it, &["statusID", "statusId"]),
                "boxTypeID" | "boxTypeId" => first_cell(it, &["boxTypeID", "boxTypeId"]),
                "statusIDName" => {
                    status_id_name(&first_cell(it, &["statusID", "statusId"]))
                }
                "boxTypeIDName" => {
                    box_type_id_name(&first_cell(it, &["boxTypeID", "boxTypeId"]))
                }
                "nmID" | "nmId" => first_cell(it, &["nmID", "nmId"]),
                "id" => first_cell(it, &["ID", "id"]),
                "createdAt" | "createDate" => {
                    first_cell(it, &["createdAt", "createDate"])
                }
                _ => json_cell(it, f.name.as_str()),
            }
        })
        .collect()
}
