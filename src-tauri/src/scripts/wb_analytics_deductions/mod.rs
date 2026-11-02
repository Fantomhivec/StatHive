//! Подмены / неверные вложения: `GET /api/analytics/v1/deductions`.

pub mod fields;

use crate::core::rate_profiles::analytics_deductions as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use self::fields::{append_rows, default_fields, total_count};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://seller-analytics-api.wildberries.ru/api/analytics/v1/deductions";
const LIMIT: i64 = 1000;

pub struct AnalyticsDeductionsScript;

impl Script for AnalyticsDeductionsScript {
    fn key(&self) -> &'static str {
        "analytics_deductions"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.analytics_deductions")
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
    ctx.log.info("Скрипт «Удержания (подмены)»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (from_dt, to_dt) = dates::resolve_period(ctx, 30)?;
    let from_s = from_dt.format("%Y-%m-%d").to_string();
    let to_s = to_dt.format("%Y-%m-%d").to_string();

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
            let from_s = from_s.clone();
            let to_s = to_s.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut rows = Vec::new();
                let mut offset = 0i64;
                loop {
                    let url = format!(
                        "{URL}?dateFrom={from_s}&dateTo={to_s}&limit={LIMIT}&offset={offset}&sort=dtBonus&order=desc"
                    );
                    let resp = fetch(ctx, &cabinet, token.trim(), &url).await?;
                    let before = rows.len();
                    append_rows(&mut rows, &fields, &cabinet, &resp);
                    let added = rows.len() - before;
                    let total = total_count(&resp).unwrap_or(i64::MAX);
                    offset += LIMIT;
                    if added == 0 || offset >= total || added < LIMIT as usize {
                        break;
                    }
                }
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
