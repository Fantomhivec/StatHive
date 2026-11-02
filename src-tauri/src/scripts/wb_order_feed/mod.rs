//! Лента заказов WB (`order_feed`).

pub mod fetch;
pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, dates};
use chrono::Duration;
use self::fields::{build_rows, default_fields};
use self::fetch::fetch_all_unique;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const MAX_WINDOW_DAYS: i64 = 31;

pub struct OrderFeedScript;

impl Script for OrderFeedScript {
    fn key(&self) -> &'static str {
        "order_feed"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.order_feed")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_order_feed(ctx).await })
    }
}

async fn run_order_feed(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Лента заказов»: старт");
    ctx.check_cancelled()?;

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (period_from, period_to) = dates::resolve_period(ctx, 14)?;
    let mut request_from = period_from;
    let request_to = period_to;
    let min_from = request_to - Duration::days(MAX_WINDOW_DAYS - 1);
    if request_from < min_from {
        ctx.log.info(format!(
            "Период {period_from}..{period_to} длиннее {MAX_WINDOW_DAYS} дн. API отдаёт максимум {MAX_WINDOW_DAYS} дн. — запрос: {min_from}..{request_to}"
        ));
        request_from = min_from;
    } else {
        ctx.log.info(format!("Период: {request_from}..{request_to}"));
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    if fields.iter().filter(|f| f.selected).count() == 0 {
        return Err("Не выбраны поля для выгрузки.".into());
    }

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
                    return Ok((cabinet, None));
                }
                match fetch_cabinet(ctx, &cabinet, token.trim(), request_from, request_to).await
                {
                    Ok(data) => Ok((cabinet, Some(data))),
                    Err(e) => {
                        ctx.log.warn(e);
                        Ok((cabinet, None))
                    }
                }
            }
        },
    )
    .await?;

    ctx.check_cancelled()?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, data) in parts {
        let Some((orders, snapshot, currency, unique)) = data else {
            continue;
        };
        total += unique;
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, &orders, &snapshot, &currency, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} заказов ленты на лист «{sheet_name}»"),
    })
}

async fn fetch_cabinet(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<(Vec<Value>, String, String, usize), String> {
    ctx.log.info(format!(
        "Кабинет {cabinet}: старт {from}..{to}"
    ));
    let (orders, snapshot, currency, duplicates) =
        fetch_all_unique(ctx, cabinet, token, from, to).await?;
    let unique = orders.len();
    ctx.log.info(format!(
        "Кабинет {cabinet}: уникальных={unique}, дублей={duplicates}"
    ));
    Ok((orders, snapshot, currency, unique))
}
