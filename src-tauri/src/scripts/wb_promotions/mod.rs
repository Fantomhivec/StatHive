//! `promotions` — GET dp-calendar `/api/v1/calendar/promotions` (raw token).

use crate::core::rate_profiles::promotions as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use crate::scripts::{common, dates};
use chrono::NaiveDate;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const PAGE: i64 = 1000;

pub struct PromotionsScript;

impl Script for PromotionsScript {
    fn key(&self) -> &'static str {
        "promotions"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.promotions")
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
    ctx.log.info("Скрипт «Список акций»: старт");
    let (from, to) = require_period(ctx)?;
    let start = format!("{}T00:00:00Z", from.format("%Y-%m-%d"));
    let end = format!("{}T23:59:59Z", to.format("%Y-%m-%d"));
    let all_promo = all_promo(ctx);
    ctx.log.info(format!(
        "Период: {start}..{end}; все акции: {}",
        if all_promo { "да" } else { "нет" }
    ));
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
    if selected.is_empty() {
        return Err("Не выбраны поля для выгрузки".into());
    }
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            let start = start.clone();
            let end = end.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                match fetch_all(ctx, &cabinet, token.trim(), &profile, &start, &end, all_promo).await
                {
                    Ok(list) => {
                        ctx.log
                            .info(format!("Кабинет «{cabinet}»: {} акций", list.len()));
                        Ok((cabinet, list))
                    }
                    Err(e) => {
                        ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                        Ok((cabinet, Vec::new()))
                    }
                }
            }
        },
    )
    .await?;

    let mut all_rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, list) in parts {
        for promo in &list {
            let row = selected
                .iter()
                .map(|f| cell(&f.name, &cabinet, promo))
                .collect();
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
        message: format!("Записано акций: {total} → «{sheet_name}»"),
    })
}

async fn fetch_all(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    start: &str,
    end: &str,
    all_promo: bool,
) -> Result<Vec<Value>, String> {
    let mut offset = 0i64;
    let mut out = Vec::new();
    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = format!(
            "https://dp-calendar-api.wildberries.ru/api/v1/calendar/promotions?startDateTime={}&endDateTime={}&allPromo={}&limit={PAGE}&offset={offset}",
            urlencoding::encode(start),
            urlencoding::encode(end),
            if all_promo { "true" } else { "false" }
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
                Auth::Raw(token),
            )
            .await?;
        let page = v
            .get("data")
            .and_then(|d| d.get("promotions"))
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();
        let n = page.len() as i64;
        out.extend(page);
        if n == 0 || n < PAGE {
            break;
        }
        offset += PAGE;
    }
    Ok(out)
}

fn all_promo(ctx: &ScriptContext) -> bool {
    for key in ["promotions_all_promo", "allPromo", "all_promo"] {
        if let Some(s) = ctx.params.get(key) {
            return matches!(
                s.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            );
        }
    }
    false
}

fn require_period(ctx: &ScriptContext) -> Result<(NaiveDate, NaiveDate), String> {
    let from = ctx
        .params
        .get("date_from")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let to = ctx
        .params
        .get("date_to")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    match (from, to) {
        (Some(a), Some(b)) => {
            let start =
                dates::parse_date(&a).ok_or_else(|| format!("Некорректная date_from «{a}»"))?;
            let end = dates::parse_date(&b).ok_or_else(|| format!("Некорректная date_to «{b}»"))?;
            if end < start {
                return Err("date_to раньше date_from".into());
            }
            Ok((start, end))
        }
        _ => Err("Укажите период (дата начала и окончания)".into()),
    }
}

fn default_fields() -> Vec<ScriptField> {
    ["id", "name", "start_date_time", "end_date_time", "type", "cabinet"]
        .into_iter()
        .map(|n| ScriptField {
            name: n.into(),
            title: crate::core::i18n::t(&format!("fields.promotions.{n}")),
            selected: true,
        })
        .collect()
}

fn cell(name: &str, cabinet: &str, promo: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "id" => scalar(promo.get("id")),
        "name" => scalar(promo.get("name")),
        "start_date_time" => stamp(promo.get("startDateTime")),
        "end_date_time" => stamp(promo.get("endDateTime")),
        "type" => scalar(promo.get("type")),
        _ => String::new(),
    }
}

fn stamp(v: Option<&Value>) -> String {
    let raw = scalar(v);
    if raw.is_empty() {
        String::new()
    } else {
        sheet_dates::format_wb_timestamp(&raw)
    }
}

fn scalar(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}
