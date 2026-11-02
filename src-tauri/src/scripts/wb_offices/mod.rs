//! `wb_offices` — GET marketplace `/api/v3/offices` → Sheets.

use crate::core::rate_profiles::wb_offices as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://marketplace-api.wildberries.ru/api/v3/offices";

pub struct WbOfficesScript;

impl Script for WbOfficesScript {
    fn key(&self) -> &'static str {
        "wb_offices"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_offices")
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
    ctx.log.info("Скрипт «Офисы WB»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let list = fetch_offices(ctx, &cabinet, token.trim(), &profile).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} офисов", list.len()));
                Ok((cabinet, list))
            }
        },
    )
    .await?;

    let mut all_rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, list) in parts {
        for it in &list {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(cell(f.name.as_str(), &cabinet, it));
            }
            all_rows.push(row);
            total += 1;
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано офисов: {total} → «{sheet_name}»"),
    })
}

async fn fetch_offices(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
) -> Result<Vec<Value>, String> {
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile,
            },
            URL,
            Auth::Raw(token),
        )
        .await?;
    Ok(match v {
        Value::Array(a) => a,
        other => other.as_array().cloned().unwrap_or_default(),
    })
}

fn default_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "id",
        "name",
        "address",
        "city",
        "longitude",
        "latitude",
        "cargoType",
        "deliveryType",
        "selected",
    ]
    .into_iter()
    .map(|n| ScriptField {
        name: n.into(),
        title: crate::core::i18n::t(&format!("fields.wb_offices.{n}")),
        selected: true,
    })
    .collect()
}

fn cell(name: &str, cabinet: &str, it: &Value) -> String {
    if name == "cabinet" {
        return cabinet.to_string();
    }
    match it.get(name) {
        Some(Value::Null) | None => String::new(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}
