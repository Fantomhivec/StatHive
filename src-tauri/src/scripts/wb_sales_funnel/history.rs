//! `sales_funnel_products_history` — статистика воронки по дням.

use crate::core::sheet_dates;
use crate::core::rate_profiles::sales_funnel_products_history as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges;
use chrono::Duration;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

const URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v3/sales-funnel/products/history";
const NM_BATCH: usize = 20;
const MAX_DAYS: i64 = 7;

pub struct SalesFunnelProductsHistoryScript;

impl Script for SalesFunnelProductsHistoryScript {
    fn key(&self) -> &'static str {
        "sales_funnel_products_history"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.sales_funnel_products_history")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> Option<&'a str> {
    ctx.params
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Воронка: история по дням»: старт");
    let (mut start, end) = crate::scripts::dates::resolve_period(ctx, 7)?;
    let min_from = end - Duration::days(MAX_DAYS - 1);
    if start < min_from {
        ctx.log.info(format!(
            "Период длиннее {MAX_DAYS} дн. — API history максимум неделя: {min_from}..{end}"
        ));
        start = min_from;
    }

    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let src_id = param(ctx, "second_spreadsheet_id")
        .or_else(|| param(ctx, "source_spreadsheet_id"))
        .unwrap_or(dest_id.as_str());
    let src_sheet = param(ctx, "second_sheet_name")
        .or_else(|| param(ctx, "source_sheet_name"))
        .unwrap_or(dest_sheet.as_str());
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    let nm_r = param(ctx, "nm_id_source_range").unwrap_or("");
    if cab_r.is_empty() || nm_r.is_empty() {
        return Err("Укажите диапазоны кабинетов и nmID.".into());
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let client = common::google_sheets_client(ctx).await?;
    let by_cab =
        sheet_ranges::load_cabinet_nm_map(&client, src_id, src_sheet, cab_r, nm_r).await?;
    if by_cab.is_empty() {
        return Ok(common::empty_data_output());
    }
    let allowed: HashSet<String> = common::resolve_cabinets(ctx)
        .into_iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let cab_order: Vec<String> = by_cab
        .keys()
        .filter(|c| allowed.is_empty() || allowed.contains(&c.to_ascii_lowercase()))
        .cloned()
        .collect();
    if cab_order.is_empty() {
        return Err("Нет nm для выбранных кабинетов".into());
    }

    let start_s = start.format("%Y-%m-%d").to_string();
    let end_s = end.format("%Y-%m-%d").to_string();
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let nms = by_cab.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let start_s = start_s.clone();
            let end_s = end_s.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let rows =
                    fetch_history(ctx, &cabinet, token.trim(), &nms, &start_s, &end_s).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} строк истории", rows.len()));
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, products) in parts {
        if products.is_empty() {
            continue;
        }
        let built = build_rows(&cabinet, &products, &fields);
        total += built.len().saturating_sub(1);
        common::append_cabinet_rows(&mut all_rows, &mut header_done, built);
    }
    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк истории воронки на лист «{dest_sheet}»"),
    })
}

async fn fetch_history(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    nms: &[i64],
    start: &str,
    end: &str,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    for chunk in nms.chunks(NM_BATCH) {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let body = json!({
            "selectedPeriod": { "start": start, "end": end },
            "nmIds": chunk,
            "skipDeletedNm": true,
            "aggregationLevel": "day",
        });
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                URL,
                Auth::Raw(token),
                &body,
            )
            .await?;
        match v {
            Value::Array(a) => all.extend(a),
            other => {
                if let Some(a) = other.as_array() {
                    all.extend(a.iter().cloned());
                } else if let Some(a) = other.pointer("/data").and_then(|x| x.as_array()) {
                    all.extend(a.iter().cloned());
                }
            }
        }
    }
    Ok(all)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("title", false),
        field("vendorCode", true),
        field("brandName", false),
        field("subjectName", false),
        field("date", true),
        field("openCount", true),
        field("cartCount", true),
        field("orderCount", true),
        field("orderSum", false),
        field("buyoutCount", true),
        field("buyoutSum", false),
        field("buyoutPercent", false),
        field("addToCartConversion", false),
        field("cartToOrderConversion", false),
        field("addToWishlistCount", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.sales_funnel_products_history.{name}")),
        selected,
    }
}

/// Разворачивает product.history[] в строки.
pub fn build_rows(cabinet: &str, products: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    for item in products {
        let product = item.get("product").unwrap_or(item);
        let history = item
            .get("history")
            .or_else(|| product.get("history"))
            .and_then(|h| h.as_array())
            .cloned()
            .unwrap_or_default();
        if history.is_empty() {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(cell(f.name.as_str(), cabinet, product, None));
            }
            rows.push(row);
            continue;
        }
        for day in &history {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(cell(f.name.as_str(), cabinet, product, Some(day)));
            }
            rows.push(row);
        }
    }
    rows
}

fn cell(name: &str, cabinet: &str, product: &Value, day: Option<&Value>) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmId" => num(product, "nmId"),
        "title" => str(product, "title"),
        "vendorCode" => str(product, "vendorCode"),
        "brandName" => str(product, "brandName"),
        "subjectName" => str(product, "subjectName"),
        "date" => {
            let raw = day.map(|d| str(d, "date")).unwrap_or_default();
            if raw.is_empty() {
                raw
            } else {
                sheet_dates::format_wb_timestamp(&raw)
            }
        }
        other => day
            .map(|d| any(d, other))
            .unwrap_or_default(),
    }
}

fn num(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
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
