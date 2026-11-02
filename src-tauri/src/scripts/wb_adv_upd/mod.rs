//! Расходы РК: `wb_adv_upd` — `GET /adv/v1/upd` → лист «Расходы».

pub mod fields;

use crate::core::rate_profiles::wb_adv_upd as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use self::fields::{
    build_chunks, default_fields, parse_upd_time, row_from_item, upd_time_sort_key,
};
use crate::scripts::common;
use crate::scripts::dates;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const UPD_URL: &str = "https://advert-api.wildberries.ru/adv/v1/upd";

/// Списания рекламы (UPD) за период.
pub struct AdvUpdExpensesScript;

impl Script for AdvUpdExpensesScript {
    fn key(&self) -> &'static str {
        "wb_adv_upd"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_upd")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_upd(ctx).await })
    }
}

async fn run_upd(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Расходы рекламных кампаний»: старт");

    let (from_dt, to_dt) = dates::resolve_period(ctx, 14)?;
    let request_from = from_dt - Duration::days(1);
    let request_to = to_dt + Duration::days(1);
    let filter_from = from_dt.and_time(NaiveTime::MIN);
    let filter_to = to_dt
        .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap_or(NaiveTime::MIN));

    ctx.log.info(format!(
        "Период: {from_dt}..{to_dt}; запрос ±1д: {request_from}..{request_to}"
    ));

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log
        .info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let chunks = build_chunks(request_from, request_to, 31);
    let wb_tokens = common::load_wb_tokens()?;
    let upd_time_idx = selected
        .iter()
        .position(|f| f.name.eq_ignore_ascii_case("updTime"));
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let chunks = chunks.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let mut items_all = Vec::new();
                for (ch_from, ch_to) in &chunks {
                    let items =
                        fetch_upd(ctx, &cabinet, token.trim(), *ch_from, *ch_to).await?;
                    items_all.extend(items);
                }
                Ok((cabinet, items_all))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> =
        vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        let mut cabinet_data: Vec<Vec<String>> = Vec::new();
        for item in &items {
            if !in_filter(item, filter_from, filter_to) {
                continue;
            }
            cabinet_data.push(row_from_item(&cabinet, item, &fields));
        }
        ctx.log.info(format!(
            "Кабинет «{cabinet}»: {} строк расходов",
            cabinet_data.len()
        ));
        total += cabinet_data.len();
        all_rows.extend(cabinet_data);
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    if let Some(idx) = upd_time_idx {
        let header = all_rows.remove(0);
        all_rows.sort_by(|a, b| {
            let ka = a.get(idx).map(|s| upd_time_sort_key(s)).unwrap_or(chrono::NaiveDate::MAX);
            let kb = b.get(idx).map(|s| upd_time_sort_key(s)).unwrap_or(chrono::NaiveDate::MAX);
            ka.cmp(&kb)
        });
        all_rows.insert(0, header);
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк на лист «{sheet_name}»"),
    })
}

fn in_filter(item: &Value, from: NaiveDateTime, to: NaiveDateTime) -> bool {
    let raw = item
        .get("updTime")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if raw.trim().is_empty() {
        return true;
    }
    match parse_upd_time(&raw) {
        Some(dt) => dt >= from && dt <= to,
        None => false,
    }
}

async fn fetch_upd(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Vec<Value>, String> {
    let url = format!(
        "{UPD_URL}?from={}&to={}",
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );
    ctx.log.info(format!(
        "GET adv/v1/upd {from}..{to} (кабинет «{cabinet}»)"
    ));
    let profile = rate::profile();
    let v = ctx
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
    if let Some(arr) = v.as_array() {
        return Ok(arr.clone());
    }
    Ok(v.get("data")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default())
}
