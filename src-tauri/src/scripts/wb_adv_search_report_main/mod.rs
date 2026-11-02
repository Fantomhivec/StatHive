//! Главная страница search-report: `POST /api/v2/search-report/report`.

pub mod fields;

use crate::core::rate_profiles::wb_adv_search_report_main as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use crate::scripts::sheet_ranges;
use self::fields::{append_report_rows, default_fields};
use chrono::Duration;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://seller-analytics-api.wildberries.ru/api/v2/search-report/report";
const PAGE: i64 = 100;

pub struct AdvSearchReportMainScript;

impl Script for AdvSearchReportMainScript {
    fn key(&self) -> &'static str {
        "wb_adv_search_report_main"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_search_report_main")
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
    ctx.log.info("Скрипт «Отчёт по поисковым запросам»: старт");
    let (from_dt, to_dt) = dates::resolve_period(ctx, 7)?;
    let cur_s = from_dt.format("%Y-%m-%d").to_string();
    let cur_e = to_dt.format("%Y-%m-%d").to_string();
    let span = (to_dt - from_dt).num_days() + 1;
    let past_e = from_dt - Duration::days(1);
    let past_s = past_e - Duration::days(span.saturating_sub(1));
    let past_s_s = past_s.format("%Y-%m-%d").to_string();
    let past_e_s = past_e.format("%Y-%m-%d").to_string();

    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let src_id = param(ctx, "second_spreadsheet_id")
        .or_else(|| param(ctx, "source_spreadsheet_id"))
        .unwrap_or(dest_id.as_str());
    let src_sheet = param(ctx, "second_sheet_name")
        .or_else(|| param(ctx, "source_sheet_name"))
        .unwrap_or(dest_sheet.as_str());
    let cab_range = param(ctx, "cabinets_source_range").unwrap_or("");
    let nm_range = param(ctx, "nm_id_source_range").unwrap_or("");

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let client = common::google_sheets_client(ctx).await?;
    let by_cab = if cab_range.is_empty() || nm_range.is_empty() {
        // Без Second — по выбранным кабинетам, без фильтра nm.
        let mut m = HashMap::new();
        for c in common::resolve_cabinets(ctx) {
            m.insert(c, Vec::<i64>::new());
        }
        m
    } else {
        let m =
            sheet_ranges::load_cabinet_nm_map(&client, src_id, src_sheet, cab_range, nm_range)
                .await?;
        ctx.log.info(format!("Кабинетов с nm: {}", m.len()));
        m
    };
    if by_cab.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let allowed: HashSet<String> = common::resolve_cabinets(ctx)
        .into_iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let cab_order: Vec<String> = by_cab
        .keys()
        .filter(|c| allowed.is_empty() || allowed.contains(&c.to_ascii_lowercase()))
        .cloned()
        .collect();

    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let nms = by_cab.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let fields = fields.clone();
            let cur_s = cur_s.clone();
            let cur_e = cur_e.clone();
            let past_s_s = past_s_s.clone();
            let past_e_s = past_e_s.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut rows = Vec::new();
                let mut offset = 0i64;
                loop {
                    let mut body = json!({
                        "currentPeriod": { "start": cur_s, "end": cur_e },
                        "pastPeriod": { "start": past_s_s, "end": past_e_s },
                        "positionCluster": "all",
                        "orderBy": { "field": "orders", "mode": "desc" },
                        "includeSubstitutedSKUs": true,
                        "includeSearchTexts": true,
                        "limit": PAGE,
                        "offset": offset
                    });
                    if !nms.is_empty() {
                        body["nmIds"] = json!(nms);
                    }
                    let resp = fetch(ctx, &cabinet, token.trim(), &body).await?;
                    let before = rows.len();
                    append_report_rows(&mut rows, &fields, &cabinet, &resp);
                    let added = rows.len() - before;
                    if added == 0 || added < PAGE as usize {
                        break;
                    }
                    offset += PAGE;
                    if offset > 5000 {
                        break;
                    }
                }
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> =
        vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (_c, rows) in parts {
        total += rows.len();
        all_rows.extend(rows);
    }
    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, dest_id.trim(), dest_sheet.trim(), all_rows)
        .await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк на лист «{dest_sheet}»"),
    })
}

async fn fetch(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    body: &Value,
) -> Result<Value, String> {
    let profile = rate::profile();
    ctx.request
        .post_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            URL,
            Auth::Raw(token),
            body,
        )
        .await
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> Option<&'a str> {
    ctx.params
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
}
