//! Отзывы WB: архив и количество за период.

mod archive;
mod fields;

use crate::core::rate_profiles::feedbacks_count as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::scripts::{common, dates};
use chrono::{Local, NaiveDate, TimeZone};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

pub use archive::FeedbacksArchiveScript;

pub struct FeedbacksCountScript;

impl Script for FeedbacksCountScript {
    fn key(&self) -> &'static str {
        "feedbacks_count"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.feedbacks_count")
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_count(ctx).await })
    }
}

async fn run_count(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Количество отзывов»: старт");
    let (from, to) = require_period(ctx)?;
    let from_unix = local_unix(from, 0, 0, 0);
    let to_unix = local_unix(to, 23, 59, 59);
    ctx.log.info(format!(
        "Период: {from}..{to} (unix {from_unix}..{to_unix})"
    ));
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
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
                    return Ok((0i64, 0i64));
                }
                let answered = match count_one(ctx, &cabinet, token.trim(), &profile, from_unix, to_unix, true)
                    .await
                {
                    Ok(n) => n,
                    Err(e) => {
                        ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                        return Ok((0, 0));
                    }
                };
                let unanswered =
                    match count_one(ctx, &cabinet, token.trim(), &profile, from_unix, to_unix, false)
                        .await
                    {
                        Ok(n) => n,
                        Err(e) => {
                            ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                            return Ok((answered, 0));
                        }
                    };
                let total = answered + unanswered;
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: обработанных={answered}, необработанных={unanswered}, всего={total}"
                ));
                Ok((answered, unanswered))
            }
        },
    )
    .await?;
    let mut answered = 0i64;
    let mut unanswered = 0i64;
    for (a, u) in parts {
        answered += a;
        unanswered += u;
    }
    let grand = answered + unanswered;
    ctx.log.info(format!(
        "Итого: обработанных={answered}, необработанных={unanswered}, всего={grand}"
    ));
    Ok(ScriptOutput {
        rows_written: 0,
        message: format!(
            "Отзывы за период: обработанных {answered}, необработанных {unanswered}, всего {grand}. В таблицу не записывалось."
        ),
    })
}

async fn count_one(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    from_unix: i64,
    to_unix: i64,
    answered: bool,
) -> Result<i64, String> {
    let url = format!(
        "https://feedbacks-api.wildberries.ru/api/v1/feedbacks/count?dateFrom={from_unix}&dateTo={to_unix}&isAnswered={}",
        if answered { "true" } else { "false" }
    );
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile,
            },
            &url,
            Auth::Bearer(token),
        )
        .await?;
    parse_count(&v)
}

fn parse_count(v: &Value) -> Result<i64, String> {
    if v.get("error").and_then(|e| e.as_bool()).unwrap_or(false) {
        let text = v
            .get("errorText")
            .and_then(|t| t.as_str())
            .unwrap_or("ошибка ответа feedbacks/count");
        return Err(text.to_string());
    }
    Ok(v.get("data").and_then(|d| d.as_i64()).or_else(|| {
        v.get("data")
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse().ok())
    }).unwrap_or(0))
}

fn require_period(ctx: &ScriptContext) -> Result<(NaiveDate, NaiveDate), String> {
    let from = ctx.params.get("date_from").map(|s| s.trim().to_string());
    let to = ctx.params.get("date_to").map(|s| s.trim().to_string());
    match (from.filter(|s| !s.is_empty()), to.filter(|s| !s.is_empty())) {
        (Some(a), Some(b)) => {
            let start = dates::parse_date(&a).ok_or_else(|| format!("Некорректная date_from «{a}»"))?;
            let end = dates::parse_date(&b).ok_or_else(|| format!("Некорректная date_to «{b}»"))?;
            if end < start {
                return Err("date_to раньше date_from".into());
            }
            Ok((start, end))
        }
        _ => Err("Укажите период (дата начала и окончания)".into()),
    }
}

fn local_unix(date: NaiveDate, h: u32, m: u32, s: u32) -> i64 {
    let naive = date.and_hms_opt(h, m, s).expect("hms");
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| naive.and_utc().timestamp())
}
