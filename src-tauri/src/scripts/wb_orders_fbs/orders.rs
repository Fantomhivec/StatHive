//! `orders_fbs` — GET marketplace `/api/v3/orders`.

use super::fields::{build_order_rows, orders_fields};
use crate::core::rate_profiles::orders_fbs as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const BASE: &str = "https://marketplace-api.wildberries.ru/api/v3/orders";
const PAGE_LIMIT: i64 = 1000;

pub struct OrdersFbsScript;

impl Script for OrdersFbsScript {
    fn key(&self) -> &'static str {
        "orders_fbs"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.orders_fbs")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        orders_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Заказы FBS»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 7)?;
    ctx.log.info(format!(
        "Период: {}..{}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    ));
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, orders_fields());
    if fields.iter().filter(|f| f.selected).count() == 0 {
        return Err("Не выбраны поля для выгрузки.".into());
    }

    let date_from = day_unix(start, false);
    let date_to = day_unix(end, true);
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
                let orders =
                    fetch_all(ctx, &cabinet, token.trim(), date_from, date_to).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} заказов", orders.len()));
                Ok((cabinet, orders))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, orders) in parts {
        if orders.is_empty() {
            continue;
        }
        total += orders.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_order_rows(&cabinet, &orders, &fields),
        );
    }
    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} заказов FBS на лист «{sheet_name}»"),
    })
}

fn day_unix(d: NaiveDate, end_of_day: bool) -> i64 {
    let t = if end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59).unwrap()
    } else {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    };
    Utc.from_utc_datetime(&d.and_time(t)).timestamp()
}

async fn fetch_all(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    date_from: i64,
    date_to: i64,
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut next: i64 = 0;
    let mut all = Vec::new();
    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = format!(
            "{BASE}?limit={PAGE_LIMIT}&next={next}&dateFrom={date_from}&dateTo={date_to}"
        );
        let mut v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                &url,
                Auth::Raw(token),
            )
            .await?;
        let orders = v
            .get_mut("orders")
            .and_then(|x| x.as_array_mut())
            .map(std::mem::take)
            .unwrap_or_default();
        let n = orders.len();
        all.extend(orders);
        let next_val = v.get("next").and_then(|x| x.as_i64()).unwrap_or(0);
        if n == 0 || next_val == 0 || next_val == next {
            break;
        }
        next = next_val;
    }
    Ok(all)
}
