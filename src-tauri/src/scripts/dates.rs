//! Общий разбор дат для скриптов (шаблоны / калькулятор).

use crate::core::script::ScriptContext;
use chrono::{Duration, Local, NaiveDate};

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(t, "%d.%m.%Y").ok())
        .or_else(|| NaiveDate::parse_from_str(t, "%d/%m/%Y").ok())
}

fn param(ctx: &ScriptContext, key: &str) -> Option<String> {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Период из `date_from`/`date_to` (алиасы `start`/`end`) или последние `default_days` до вчера.
pub fn resolve_period(
    ctx: &ScriptContext,
    default_days: i64,
) -> Result<(NaiveDate, NaiveDate), String> {
    let from = param(ctx, "date_from").or_else(|| param(ctx, "start"));
    let to = param(ctx, "date_to").or_else(|| param(ctx, "end"));
    match (from, to) {
        (Some(a), Some(b)) => {
            let start = parse_date(&a).ok_or_else(|| format!("Некорректная date_from «{a}»"))?;
            let end = parse_date(&b).ok_or_else(|| format!("Некорректная date_to «{b}»"))?;
            if end < start {
                return Err("date_to раньше date_from".into());
            }
            Ok((start, end))
        }
        _ => {
            let end = Local::now().date_naive() - Duration::days(1);
            let start = end - Duration::days(default_days.saturating_sub(1));
            Ok((start, end))
        }
    }
}

/// Одна дата из `date_from` / `date` или сегодня.
pub fn resolve_single_date(ctx: &ScriptContext) -> Result<NaiveDate, String> {
    match param(ctx, "date_from").or_else(|| param(ctx, "date")) {
        None => Ok(Local::now().date_naive()),
        Some(s) => parse_date(&s).ok_or_else(|| {
            format!("Некорректная дата «{s}». Ожидается YYYY-MM-DD или dd.MM.yyyy")
        }),
    }
}
