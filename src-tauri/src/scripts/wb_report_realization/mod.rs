//! Финансовый отчёт детализации (`report_realization_v5`).

pub mod fields;

use crate::core::rate_profiles::report_realization_v5 as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::wb_finance_util::{self, resolve_period};
use self::fields::default_fields;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://finance-api.wildberries.ru/api/finance/v1/sales-reports/detailed";
const LIMIT: i64 = 100_000;

pub struct ReportRealizationV5Script;

impl Script for ReportRealizationV5Script {
    fn key(&self) -> &'static str {
        "report_realization_v5"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.report_realization_v5")
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
    ctx.log.info("Старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 14)?;
    let period = resolve_period(ctx.params.get("report_realization_v5_period").map(|s| s.as_str()));
    ctx.log.info(format!(
        "Период {}..{} ({period})",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    ));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let names: Vec<&str> = selected.iter().map(|f| f.name.as_str()).collect();

    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Кабинет «{cabinet}»: нет токена, пропуск"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let items =
                    fetch_detailed(ctx, &cabinet, token.trim(), start, end, period).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} строк", items.len()));
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows = vec![common::selected_titles(&fields)];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        if items.is_empty() {
            continue;
        }
        total += items.len();
        all_rows.extend(wb_finance_util::build_data_rows(&cabinet, &items, &names));
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк на лист «{sheet_name}»"),
    })
}

async fn fetch_detailed(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    period: &str,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    let mut rrd_id: i64 = 0;
    let mut page = 0u32;
    loop {
        page += 1;
        let body = json!({
            "dateFrom": start.format("%Y-%m-%d").to_string(),
            "dateTo": end.format("%Y-%m-%d").to_string(),
            "limit": LIMIT,
            "rrdId": rrd_id,
            "period": period,
            "fields": [],
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
                Auth::Bearer(token),
                &body,
            )
            .await?;
        let batch = match v {
            Value::Array(a) => a,
            other => other
                .as_array()
                .cloned()
                .unwrap_or_default(),
        };
        ctx.log
            .info(format!("Страница {page}: {} строк", batch.len()));
        if batch.is_empty() {
            break;
        }
        let n = batch.len() as i64;
        let last_rrd = batch
            .last()
            .and_then(|x| x.get("rrdId"))
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        all.extend(batch);
        if page == 1 && n < LIMIT {
            break;
        }
        if last_rrd == 0 || last_rrd == rrd_id {
            break;
        }
        rrd_id = last_rrd;
        tokio::time::sleep(std::time::Duration::from_secs(61)).await;
    }
    Ok(all)
}
