//! `prices_history_task` — детализация uploadID.

use crate::core::rate_profiles::prices_history_task as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name, parse_i64_cell};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const BASE: &str = "https://discounts-prices-api.wildberries.ru/api/v2/history/goods/task";
const LIMIT: i64 = 1000;

pub struct PricesHistoryTaskScript;

impl Script for PricesHistoryTaskScript {
    fn key(&self) -> &'static str {
        "prices_history_task"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.prices_history_task")
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
    ctx.log.info("Старт");
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
    let up_r = param(ctx, "prices_history_task_upload_id_range").unwrap_or("");
    if cab_r.is_empty() || up_r.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов и uploadID".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(&client, src_id, src_sheet, &[cab_r, up_r]).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let ups = cols.get(1).cloned().unwrap_or_default();
    let n = cabs.len().max(ups.len());

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut all_rows = vec![common::selected_titles(&fields)];
    let wb_tokens = common::load_wb_tokens()?;
    let mut total = 0usize;

    for i in 0..n {
        let cab_raw = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let up_raw = ups.get(i).map(|s| s.as_str()).unwrap_or("");
        if cab_raw.is_empty() {
            continue;
        }
        let Some(upload_id) = parse_i64_cell(up_raw).filter(|&x| x > 0) else {
            continue;
        };
        let label = normalize_cabinet_name(cab_raw);
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &label);
        if token.trim().is_empty() {
            ctx.log.warn(format!("Кабинет «{label}»: нет токена"));
            continue;
        }
        let items = fetch_history(ctx, &label, token.trim(), upload_id).await?;
        for it in &items {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(hist_cell(f.name.as_str(), &label, upload_id, it));
            }
            all_rows.push(row);
            total += 1;
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк истории на лист «{dest_sheet}»"),
    })
}

async fn fetch_history(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    upload_id: i64,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let url = format!("{BASE}?uploadID={upload_id}&limit={LIMIT}&offset={offset}");
        let v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                &url,
                Auth::Bearer(token),
            )
            .await?;
        let items = v
            .pointer("/data/historyGoods")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let n = items.len() as i64;
        if n == 0 {
            break;
        }
        all.extend(items);
        offset += n;
        if n < LIMIT {
            break;
        }
    }
    Ok(all)
}

fn hist_cell(name: &str, cabinet: &str, upload_id: i64, it: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "uploadID" => upload_id.to_string(),
        "status" => match it.get("status").and_then(|v| v.as_i64()) {
            Some(2) => "Успешно".into(),
            Some(3) => "Ошибка".into(),
            Some(n) => n.to_string(),
            None => json_str(it, "status"),
        },
        "nmID" => json_num(it, "nmID"),
        "vendorCode" => json_str(it, "vendorCode"),
        "sizeID" => json_num(it, "sizeID"),
        "techSizeName" => json_str(it, "techSizeName"),
        "price" => json_num(it, "price"),
        "currencyIsoCode4217" => json_str(it, "currencyIsoCode4217"),
        "discount" => json_num(it, "discount"),
        "clubDiscount" => json_num(it, "clubDiscount"),
        "errorText" => json_str(it, "errorText"),
        _ => String::new(),
    }
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_num(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("uploadID", true),
        field("nmID", true),
        field("vendorCode", true),
        field("sizeID", true),
        field("techSizeName", true),
        field("price", true),
        field("currencyIsoCode4217", true),
        field("discount", true),
        field("clubDiscount", false),
        field("status", true),
        field("errorText", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.prices_history_task.{name}")),
        selected,
    }
}
