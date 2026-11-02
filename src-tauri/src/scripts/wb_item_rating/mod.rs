pub mod fields;

use crate::core::rate_profiles::item_rating as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_rows, default_fields, PeriodMeta};
use chrono::{Duration, Local, NaiveDate};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const RATING_URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v2/item-rating";
const PAGE_LIMIT: i64 = 1000;

/// Рейтинг товаров / отзывы (`item_rating`) → Google Sheets.
pub struct ItemRatingScript;

impl Script for ItemRatingScript {
    fn key(&self) -> &'static str {
        "item_rating"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.item_rating")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_item_rating(ctx).await })
    }
}

async fn run_item_rating(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Рейтинг товаров»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (start, end) = crate::scripts::dates::resolve_period(ctx, 14)?;
    validate_period(start, end)?;
    let cur_s = start.format("%Y-%m-%d").to_string();
    let cur_e = end.format("%Y-%m-%d").to_string();
    let (past_s, past_e) = past_period(start, end);
    ctx.log.info(format!("Период: {cur_s}..{cur_e}"));
    if let (Some(ps), Some(pe)) = (&past_s, &past_e) {
        ctx.log.info(format!("Прошлый период: {ps}..{pe}"));
    }

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
            let cur_s = cur_s.clone();
            let cur_e = cur_e.clone();
            let past_s = past_s.clone();
            let past_e = past_e.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let pages = fetch_all(
                    ctx,
                    &cabinet,
                    token.trim(),
                    &cur_s,
                    &cur_e,
                    past_s.as_deref(),
                    past_e.as_deref(),
                )
                .await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: накоплено строк по кабинету"));
                Ok((cabinet, pages))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, pages) in parts {
        for data in pages {
            let meta = PeriodMeta {
                cabinet: &cabinet,
                current_start: &cur_s,
                current_end: &cur_e,
                past_start: past_s.as_deref().unwrap_or(""),
                past_end: past_e.as_deref().unwrap_or(""),
            };
            let rows = build_rows(&meta, &data, &fields);
            let n = rows.len().saturating_sub(1);
            total += n;
            common::append_cabinet_rows(&mut all_rows, &mut header_done, rows);
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} товаров рейтинга на лист «{sheet_name}»"),
    })
}

fn validate_period(start: NaiveDate, end: NaiveDate) -> Result<(), String> {
    if start > end {
        return Err("Дата начала позже окончания".into());
    }
    let days = (end - start).num_days() + 1;
    if days > 365 {
        return Err("Максимум 365 дней для периода".into());
    }
    let yesterday = Local::now().date_naive() - Duration::days(1);
    if end > yesterday {
        return Err(format!(
            "Окончание периода не позже вчера ({})",
            yesterday.format("%Y-%m-%d")
        ));
    }
    let earliest = yesterday - Duration::days(364);
    if start < earliest {
        return Err(format!(
            "Начало периода не ранее {}",
            earliest.format("%Y-%m-%d")
        ));
    }
    Ok(())
}

fn past_period(start: NaiveDate, end: NaiveDate) -> (Option<String>, Option<String>) {
    let len = end - start;
    let past_end = start - Duration::days(1);
    let past_start = past_end - len;
    let yesterday = Local::now().date_naive() - Duration::days(1);
    let earliest = yesterday - Duration::days(364);
    if past_start < earliest {
        return (None, None);
    }
    (
        Some(past_start.format("%Y-%m-%d").to_string()),
        Some(past_end.format("%Y-%m-%d").to_string()),
    )
}

async fn fetch_all(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    cur_s: &str,
    cur_e: &str,
    past_s: Option<&str>,
    past_e: Option<&str>,
) -> Result<Vec<Value>, String> {
    let mut pages = Vec::new();
    let mut offset = 0i64;
    let profile = rate::profile();
    loop {
        let mut body = json!({
            "currentPeriod": { "start": cur_s, "end": cur_e },
            "isNotIncludeNmsWithoutSales": false,
            "onlyShadowedNms": false,
            "orderBy": { "field": "feedbackRating", "mode": "desc" },
            "limit": PAGE_LIMIT,
            "offset": offset,
        });
        if let (Some(ps), Some(pe)) = (past_s, past_e) {
            body["pastPeriod"] = json!({ "start": ps, "end": pe });
        }
        ctx.log.info(format!("POST item-rating offset={offset}"));
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                RATING_URL,
                Auth::Raw(token),
                &body,
            )
            .await?;
        let data = v.get("data").cloned().unwrap_or(json!({}));
        let count = data
            .get("items")
            .and_then(|a| a.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        pages.push(data);
        if count == 0 || (count as i64) < PAGE_LIMIT {
            break;
        }
        offset += PAGE_LIMIT;
    }
    Ok(pages)
}

