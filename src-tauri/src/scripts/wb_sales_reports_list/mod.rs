//! Список отчётов реализации (`sales_reports_list`).

use crate::core::rate_profiles::report_realization_v5 as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::wb_finance_util::{self, resolve_period};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://finance-api.wildberries.ru/api/finance/v1/sales-reports/list";
const LIMIT: i64 = 1000;

pub struct SalesReportsListScript;

impl Script for SalesReportsListScript {
    fn key(&self) -> &'static str {
        "sales_reports_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.sales_reports_list")
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
                let items = fetch_list(ctx, &cabinet, token.trim(), start, end, period).await?;
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
        message: format!("Записано {total} отчётов на лист «{sheet_name}»"),
    })
}

async fn fetch_list(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    period: &str,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let body = json!({
            "dateFrom": start.format("%Y-%m-%d").to_string(),
            "dateTo": end.format("%Y-%m-%d").to_string(),
            "limit": LIMIT,
            "offset": offset,
            "period": period,
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
            other => other.as_array().cloned().unwrap_or_default(),
        };
        let n = batch.len() as i64;
        if n == 0 {
            break;
        }
        all.extend(batch);
        offset += n;
        if n < LIMIT {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(61)).await;
    }
    Ok(all)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("reportId", true),
        field("sellerFinanceName", true),
        field("dateFrom", true),
        field("dateTo", true),
        field("createDate", true),
        field("currency", true),
        field("reportType", true),
        field("reportTypeName", true),
        field("retailAmountSum", true),
        field("forPaySum", true),
        field("avgSalePercent", true),
        field("deliveryServiceSum", true),
        field("paidStorageSum", true),
        field("paidAcceptanceSum", true),
        field("deductionSum", true),
        field("penaltySum", true),
        field("additionalPaymentSum", true),
        field("cashbackAmountSum", true),
        field("cashbackDiscountSum", true),
        field("cashbackCommissionChangeSum", true),
        field("paymentSchedule", true),
        field("bankPaymentSum", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.sales_reports_list.{name}")),
        selected,
    }
}
