pub mod fields;
pub mod history;

pub use history::SalesFunnelProductsHistoryScript;

use crate::core::rate_profiles::sales_funnel_products as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_rows, default_fields};
use chrono::{Duration, NaiveDate};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const FUNNEL_URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v3/sales-funnel/products";
const PAGE_LIMIT: i64 = 1000;

/// Воронка продаж за период (`sales_funnel_products`) → Google Sheets.
pub struct SalesFunnelProductsScript;

impl Script for SalesFunnelProductsScript {
    fn key(&self) -> &'static str {
        "sales_funnel_products"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.sales_funnel_products")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_sales_funnel(ctx).await })
    }
}

async fn run_sales_funnel(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Воронка продаж за период»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log
        .info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let (start, end) = crate::scripts::dates::resolve_period(ctx, 14)?;
    if (end - start).num_days() > 365 {
        return Err("Максимум 365 дней для выбранного периода.".into());
    }
    ctx.log.info(format!(
        "Период: {}..{}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    ));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let products = fetch_all_pages(ctx, &cabinet, token.trim(), start, end).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} карточек", products.len()));
                Ok((cabinet, products))
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
        total += products.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, &products, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} карточек воронки на лист «{sheet_name}»"),
    })
}

fn build_body(start: NaiveDate, end: NaiveDate, page: i64) -> Value {
    let span = end - start;
    let past_end = start - Duration::days(1);
    let past_start = past_end - span;
    json!({
        "selectedPeriod": {
            "start": start.format("%Y-%m-%d").to_string(),
            "end": end.format("%Y-%m-%d").to_string(),
        },
        "pastPeriod": {
            "start": past_start.format("%Y-%m-%d").to_string(),
            "end": past_end.format("%Y-%m-%d").to_string(),
        },
        "nmIds": [],
        "brandNames": [],
        "subjectIds": [],
        "tagIds": [],
        "skipDeletedNm": true,
        "orderBy": { "field": "openCard", "mode": "desc" },
        "limit": PAGE_LIMIT,
        "offset": page * PAGE_LIMIT,
    })
}

async fn fetch_all_pages(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut page = 0i64;
    let profile = rate::profile();
    loop {
        let body = build_body(start, end, page);
        ctx.log.info(format!(
            "POST sales-funnel/products page={page} offset={}",
            page * PAGE_LIMIT
        ));
        let mut v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                FUNNEL_URL,
                Auth::Raw(token),
                &body,
            )
            .await?;
        let products = v
            .pointer_mut("/data/products")
            .and_then(|p| p.as_array_mut())
            .map(std::mem::take)
            .unwrap_or_default();
        let n = products.len();
        ctx.log.info(format!("Страница {page}: {n} карточек"));
        all.extend(products);
        if (n as i64) < PAGE_LIMIT {
            break;
        }
        page += 1;
    }
    Ok(all)
}
