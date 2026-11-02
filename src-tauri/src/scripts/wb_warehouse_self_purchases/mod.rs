//! Самовыкупы: `GET /api/v1/analytics/antifraud-details`.

use crate::core::rate_profiles::warehouse_self_purchases as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/v1/analytics/antifraud-details";

pub struct WarehouseSelfPurchasesScript;

impl Script for WarehouseSelfPurchasesScript {
    fn key(&self) -> &'static str {
        "warehouse_self_purchases"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouse_self_purchases")
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

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Самовыкупы»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (from_dt, _to) = dates::resolve_period(ctx, 7)?;
    let date_q = from_dt.format("%Y-%m-%d").to_string();

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let fields = fields.clone();
            let date_q = date_q.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let url = format!("{URL}?date={date_q}");
                let resp = fetch(ctx, &cabinet, token.trim(), &url).await?;
                let mut rows = Vec::new();
                append_rows(&mut rows, &fields, &cabinet, &resp);
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> =
        vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (_c, rows) in parts {
        total += rows.len();
        all_rows.extend(rows);
    }
    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    let client = common::google_sheets_client(ctx).await?;
    common::write_sheet_rows_with(ctx, &client, spreadsheet_id.trim(), sheet_name.trim(), all_rows)
        .await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк на лист «{sheet_name}»"),
    })
}

async fn fetch(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    url: &str,
) -> Result<Value, String> {
    let profile = rate::profile();
    ctx.request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            url,
            Auth::Raw(token),
        )
        .await
}

// --- fields (merged) ---
use crate::core::sheet_dates;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("sum", true),
        field("currency", true),
        field("dateFrom", true),
        field("dateTo", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.warehouse_self_purchases.{name}")),
        selected,
    }
}

pub fn append_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let details = root
        .get("details")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    for d in &details {
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "nmId" => first_num(d, &["nmID", "nmId", "nm_id"]),
                    "sum" => first_num(d, &["sum"]),
                    "currency" => first_str(d, &["currency"]),
                    "dateFrom" => fmt_date(first_str(d, &["dateFrom", "date_from"])),
                    "dateTo" => fmt_date(first_str(d, &["dateTo", "date_to"])),
                    _ => String::new(),
                })
                .collect(),
        );
    }
}

fn fmt_date(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return sheet_dates::format_date(d);
    }
    sheet_dates::format_wb_timestamp(t)
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(Value::String(s)) = v.get(*k) {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    String::new()
}

fn first_num(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match v.get(*k) {
            Some(Value::Number(n)) => return n.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() => return s.clone(),
            _ => {}
        }
    }
    String::new()
}
