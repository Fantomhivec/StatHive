//! `wb_stock_fbo_report` — остатки на складах WB.

use crate::core::rate_profiles::wb_stock_fbo_report as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::wb_stocks;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v1/stocks-report/wb-warehouses";
const NM_BATCH: usize = 1000;
const PAGE_LIMIT: i64 = 250_000;

pub struct WbStockFboReportScript;

impl Script for WbStockFboReportScript {
    fn key(&self) -> &'static str {
        "wb_stock_fbo_report"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_stock_fbo_report")
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
    ctx.log.info("Остатки FBO (склады WB): старт");
    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(_) => {
            return Ok(ScriptOutput {
                rows_written: 0,
                message: "Укажите таблицу и лист назначения".into(),
            });
        }
    };
    let src_id = param(ctx, "source_spreadsheet_id")
        .or_else(|| param(ctx, "second_spreadsheet_id"))
        .unwrap_or(dest_id.as_str());
    let src_sheet = param(ctx, "source_sheet_name")
        .or_else(|| param(ctx, "second_sheet_name"))
        .unwrap_or("Массив универсал");

    let fields = common::resolve_fields(ctx, default_fields());
    let client = common::google_sheets_client(ctx).await?;
    let by_cabinet = wb_stocks::load_filters(ctx, &client, src_id.trim(), src_sheet.trim()).await?;
    if by_cabinet.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет строк с кабинетом, nmId и chrtId в источнике".into(),
        });
    }

    let wb_tokens = common::load_wb_tokens()?;
    let cab_order: Vec<String> = by_cabinet.keys().cloned().collect();
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let filter = by_cabinet.get(&cabinet).cloned();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                let Some(filter) = filter else {
                    return Ok((cabinet, Vec::new()));
                };
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::new()));
                }
                let items = fetch_all(ctx, &cabinet, token.trim(), &filter).await?;
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows = vec![build_header(&fields)];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        for it in items {
            all_rows.push(build_data_row(&cabinet, &it, &fields));
            total += 1;
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк остатков FBO на «{dest_sheet}»"),
    })
}

async fn fetch_all(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    filter: &wb_stocks::CabinetFilter,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let nm_list: Vec<i64> = filter.nm_ids.iter().copied().collect();
    let chrt_list: Vec<i64> = filter.chrt_ids.iter().copied().collect();
    let mut all = Vec::new();
    for chunk in nm_list.chunks(NM_BATCH) {
        let mut offset = 0i64;
        loop {
            ctx.check_cancelled().map_err(|e| e.to_string())?;
            let body = json!({
                "nmIds": chunk,
                "chrtIds": &chrt_list,
                "limit": PAGE_LIMIT,
                "offset": offset,
            });
            let mut v = ctx
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
            let items = v
                .pointer_mut("/data/items")
                .and_then(|x| x.as_array_mut())
                .map(std::mem::take)
                .unwrap_or_default();
            let n = items.len() as i64;
            all.extend(items);
            if n < PAGE_LIMIT {
                break;
            }
            offset += PAGE_LIMIT;
        }
    }
    Ok(all)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("chrtId", true),
        field("warehouseId", true),
        field("warehouseName", true),
        field("regionName", false),
        field("quantity", true),
        field("inWayToClient", false),
        field("inWayFromClient", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_stock_fbo_report.{name}")),
        selected,
    }
}

pub fn build_header(fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| f.title.clone())
        .collect()
}

pub fn build_data_row(cabinet: &str, item: &Value, fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| cell(f.name.as_str(), cabinet, item))
        .collect()
}

fn cell(name: &str, cabinet: &str, item: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmId" => num(item, "nmId"),
        "chrtId" => num(item, "chrtId"),
        "warehouseId" => num(item, "warehouseId"),
        "warehouseName" => str(item, "warehouseName"),
        "regionName" => str(item, "regionName"),
        "quantity" => num(item, "quantity"),
        "inWayToClient" => num(item, "inWayToClient"),
        "inWayFromClient" => num(item, "inWayFromClient"),
        _ => String::new(),
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
