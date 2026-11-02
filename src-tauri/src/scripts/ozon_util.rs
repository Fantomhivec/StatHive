//! Общие helpers Ozon-скриптов (ID из листа, даты, yes/no).

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::script::ScriptContext;
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use crate::scripts::common;
use chrono::NaiveDate;
use serde_json::Value;
use std::collections::HashMap;

pub fn yes_no(v: Option<bool>) -> String {
    if v == Some(true) {
        "Да".into()
    } else {
        "Нет".into()
    }
}

pub fn json_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => yes_no(Some(*b)),
        Some(other) => other.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

/// `sku` | `offer_id` | `product_id` из params (default sku для calc).
pub fn resolve_id_mode(ctx: &ScriptContext) -> String {
    ctx.params
        .get("id_mode")
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "sku".into())
}

fn id_range_for_mode(ctx: &ScriptContext, mode: &str) -> String {
    let named = match mode {
        "offer_id" => ctx.params.get("offer_id_source_range"),
        "product_id" => ctx.params.get("product_id_source_range"),
        _ => ctx.params.get("sku_source_range"),
    };
    named
        .or_else(|| ctx.params.get("id_source_range"))
        .or_else(|| ctx.params.get("sku_source_range"))
        .cloned()
        .unwrap_or_else(|| "B2:B".into())
        .trim()
        .to_string()
}

pub fn source_spreadsheet(ctx: &ScriptContext, dest_id: &str) -> (String, String) {
    let sid = ctx
        .params
        .get("source_spreadsheet_id")
        .or_else(|| ctx.params.get("second_spreadsheet_id"))
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| dest_id.to_string());
    let sheet = ctx
        .params
        .get("source_sheet_name")
        .or_else(|| ctx.params.get("second_sheet_name"))
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Ozon отчёт товары универсал".into());
    (sid, sheet)
}

/// Уникальные ID из листа через уже созданный Sheets-клиент.
pub async fn read_identifiers_with(
    ctx: &ScriptContext,
    client: &GoogleSheetsClient,
    dest_spreadsheet_id: &str,
    id_mode_override: Option<&str>,
) -> Result<Vec<String>, String> {
    let mode = id_mode_override
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| resolve_id_mode(ctx));
    let range = id_range_for_mode(ctx, &mode);
    let (src_id, src_sheet) = source_spreadsheet(ctx, dest_spreadsheet_id);
    ctx.log.info(format!(
        "Чтение идентификаторов: {src_id} / «{src_sheet}»!{range}"
    ));
    let a1 = format!("'{}'!{range}", src_sheet.replace('\'', "''"));
    let rows = client
        .read_values(src_id.trim(), &a1)
        .await
        .map_err(|e| format!("Чтение источника: {e}"))?;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut skipped_zero = 0usize;
    for row in rows {
        for cell in row {
            let t = cell.trim();
            if t.is_empty() || seen.contains(t) {
                continue;
            }
            if common::is_zero_numeric_id(t) {
                skipped_zero += 1;
                continue;
            }
            let owned = t.to_string();
            seen.insert(owned.clone());
            out.push(owned);
        }
    }
    if skipped_zero > 0 {
        ctx.log.info(format!(
            "Пропущено идентификаторов sku/product_id = 0: {skipped_zero}."
        ));
    }
    Ok(out)
}

/// Уникальные непустые значения из диапазона листа-источника.
pub async fn read_identifiers(
    ctx: &ScriptContext,
    dest_spreadsheet_id: &str,
) -> Result<Vec<String>, String> {
    let client = common::google_sheets_client(ctx).await?;
    read_identifiers_with(ctx, &client, dest_spreadsheet_id, None).await
}

pub fn resolve_period_days(
    ctx: &ScriptContext,
    default_days: i64,
) -> Result<(NaiveDate, NaiveDate), String> {
    crate::scripts::dates::resolve_period(ctx, default_days)
}

pub fn to_rfc3339_start(d: NaiveDate) -> String {
    format!("{}T00:00:00.000Z", d.format("%Y-%m-%d"))
}

pub fn to_rfc3339_end(d: NaiveDate) -> String {
    format!("{}T23:59:59.999Z", d.format("%Y-%m-%d"))
}

pub fn format_ozon_dt(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return sheet_dates::format_datetime(dt.naive_utc());
    }
    raw.to_string()
}

pub const ID_BATCH: usize = 1000;

pub fn identifier_batches(ids: &[String]) -> Vec<Vec<String>> {
    ids.chunks(ID_BATCH)
        .map(|c| c.to_vec())
        .collect()
}

pub fn parse_i64_list(ids: &[String]) -> Vec<i64> {
    ids.iter()
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .filter(|&n| n > 0)
        .collect()
}

/// Filter/body id fields по режиму (list root или attributes `filter`).
pub fn id_filter_json(id_mode: &str, batch: &[String]) -> Value {
    match id_mode {
        "offer_id" => serde_json::json!({ "offer_id": batch }),
        "product_id" => serde_json::json!({ "product_id": parse_i64_list(batch) }),
        _ => serde_json::json!({ "sku": parse_i64_list(batch) }),
    }
}

/// Тело POST `/v3/product/info/list` по режиму id (stocks / product_info).
pub fn product_info_list_body(id_mode: &str, batch: &[String]) -> Value {
    id_filter_json(id_mode, batch)
}

#[derive(Clone, Default)]
pub struct ProductIds {
    pub offer_id: String,
    pub product_id: String,
    pub sku: String,
}

/// ID из item + fallback source_id по id_mode.
pub fn fill_from_item_and_source(
    item: &Value,
    id_mode: &str,
    source_id: Option<&str>,
) -> ProductIds {
    let mut ids = ProductIds {
        offer_id: json_str(item.get("offer_id")),
        product_id: json_str(item.get("id").or_else(|| item.get("product_id"))),
        sku: json_str(item.get("sku")),
    };
    if let Some(src) = source_id {
        if id_mode == "offer_id" && ids.offer_id.is_empty() {
            ids.offer_id = src.to_string();
        } else if id_mode == "product_id" && ids.product_id.is_empty() {
            ids.product_id = src.to_string();
        } else if id_mode == "sku" && ids.sku.is_empty() {
            ids.sku = src.to_string();
        }
    }
    ids
}

pub fn row_from_map(map: &HashMap<String, String>, fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| map.get(&f.name).cloned().unwrap_or_default())
        .collect()
}

/// Кэш sku→product_id на один run (ключ: cabinet|batch-joined).
pub struct SkuResolveCache {
    inner: HashMap<String, (Vec<String>, HashMap<String, String>)>,
}

impl SkuResolveCache {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub async fn resolve(
        &mut self,
        ctx: &ScriptContext,
        client_id: &str,
        api_key: &str,
        skus: &[String],
        cabinet: &str,
    ) -> Result<(Vec<String>, HashMap<String, String>), String> {
        let key = format!("{cabinet}|{}", skus.join("\u{1f}"));
        if let Some(hit) = self.inner.get(&key) {
            return Ok(hit.clone());
        }
        let resolved = resolve_sku_to_product_ids(ctx, client_id, api_key, skus, cabinet).await?;
        self.inner.insert(key, resolved.clone());
        Ok(resolved)
    }
}

/// Цена для листа: точка → запятая (паритет).
pub fn sheet_price(raw: &str) -> String {
    raw.replace('.', ",")
}

/// Число с 2 знаками, запятая как разделитель (паритет `F2` ru-RU).
pub fn f2_ru(v: Option<&Value>) -> String {
    let Some(n) = v.and_then(|x| x.as_f64()).or_else(|| {
        v.and_then(|x| x.as_str())
            .and_then(|s| s.replace(',', ".").parse::<f64>().ok())
    }) else {
        return String::new();
    };
    format!("{n:.2}").replace('.', ",")
}

pub fn param_bool(ctx: &ScriptContext, key: &str) -> bool {
    ctx.params
        .get(key)
        .map(|s| {
            let t = s.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes" || t == "on"
        })
        .unwrap_or(false)
}

pub fn param_or(ctx: &ScriptContext, key: &str, default: &str) -> String {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// GET Ozon через RequestSender (лимит + 429).
pub async fn ozon_get(
    ctx: &ScriptContext,
    cabinet: &str,
    scope: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    path: &str,
    client_id: &str,
    api_key: &str,
) -> Result<Value, String> {
    use crate::core::request_sender::Limit;
    ctx.request
        .ozon_get(
            Limit {
                cabinet,
                scope,
                profile,
            },
            path,
            client_id,
            api_key,
        )
        .await
}

/// POST Ozon через RequestSender (лимит + 429).
pub async fn ozon_post(
    ctx: &ScriptContext,
    cabinet: &str,
    scope: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    path: &str,
    client_id: &str,
    api_key: &str,
    body: &Value,
) -> Result<Value, String> {
    use crate::core::request_sender::Limit;
    ctx.request
        .ozon_post(
            Limit {
                cabinet,
                scope,
                profile,
            },
            path,
            client_id,
            api_key,
            body,
        )
        .await
}

/// sku → (product_id[], product_id → sku).
pub async fn resolve_sku_to_product_ids(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    skus: &[String],
    cabinet: &str,
) -> Result<(Vec<String>, std::collections::HashMap<String, String>), String> {
    let sku_nums = parse_i64_list(skus);
    if sku_nums.is_empty() {
        return Ok((Vec::new(), std::collections::HashMap::new()));
    }
    let body = serde_json::json!({ "sku": sku_nums });
    let profile = crate::core::rate_profiles::ozon::product_list::profile();
    let v = ozon_post(
        ctx,
        cabinet,
        crate::core::rate_profiles::ozon::product_list::SCOPE,
        &profile,
        "/v3/product/info/list",
        client_id,
        api_key,
        &body,
    )
    .await?;
    let mut ids = Vec::new();
    let mut map = std::collections::HashMap::new();
    if let Some(items) = v.get("items").and_then(|a| a.as_array()) {
        for item in items {
            let pid = item
                .get("id")
                .or_else(|| item.get("product_id"))
                .and_then(|x| x.as_i64().or_else(|| x.as_u64().map(|n| n as i64)))
                .map(|n| n.to_string())
                .unwrap_or_default();
            if pid.is_empty() {
                continue;
            }
            ids.push(pid.clone());
            let sku = item
                .get("sku")
                .and_then(|x| x.as_i64().or_else(|| x.as_u64().map(|n| n as i64)))
                .map(|n| n.to_string())
                .or_else(|| item.get("sku").and_then(|x| x.as_str()).map(str::to_string))
                .unwrap_or_default();
            if !sku.is_empty() && !map.contains_key(&pid) {
                map.insert(pid, sku);
            }
        }
    }
    ids.sort();
    ids.dedup();
    Ok((ids, map))
}

pub fn format_number_cell(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => yes_no(Some(*b)),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Ozon money object `{ amount, currency }` (FBO v3 / FBS v4) → (amount, currency); иначе пусто.
pub fn money_amount_currency(v: Option<&Value>) -> (String, String) {
    let Some(obj) = v.and_then(|x| x.as_object()) else {
        return (String::new(), String::new());
    };
    (
        json_str(obj.get("amount")),
        json_str(obj.get("currency")),
    )
}

pub fn join_str_array(v: Option<&Value>, sep: &str) -> String {
    let Some(Value::Array(a)) = v else {
        return String::new();
    };
    a.iter()
        .map(|x| json_str(Some(x)))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}
