//! Статистика поисковых кластеров: `POST /adv/v0/normquery/stats`.

pub mod fields;

use crate::core::rate_profiles::wb_adv_norm_query_stats as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use crate::scripts::sheet_ranges;
use self::fields::{append_stats_rows, default_fields};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const STATS_URL: &str = "https://advert-api.wildberries.ru/adv/v0/normquery/stats";
const BATCH: usize = 100;

pub struct AdvNormQueryStatsScript;

impl Script for AdvNormQueryStatsScript {
    fn key(&self) -> &'static str {
        "wb_adv_norm_query_stats"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_norm_query_stats")
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
    ctx.log.info("Скрипт «Статистика поисковых кластеров»: старт");
    let (from_dt, to_dt) = dates::resolve_period(ctx, 14)?;
    let from_s = from_dt.format("%Y-%m-%d").to_string();
    let to_s = to_dt.format("%Y-%m-%d").to_string();

    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let src_id = param(ctx, "second_spreadsheet_id")
        .or_else(|| param(ctx, "source_spreadsheet_id"))
        .unwrap_or(dest_id.as_str());
    let src_sheet = param(ctx, "second_sheet_name")
        .or_else(|| param(ctx, "source_sheet_name"))
        .unwrap_or("Кампании");
    let cab_range = param(ctx, "cabinets_source_range").unwrap_or("");
    let id_range = param(ctx, "id_adv_campaign_source_range").unwrap_or("");
    let nm_range = param(ctx, "nm_id_source_range").unwrap_or("");
    if cab_range.is_empty() || id_range.is_empty() {
        return Err(
            "Укажите диапазоны кабинетов и ID кампаний (и при необходимости nmID).".into(),
        );
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let client = common::google_sheets_client(ctx).await?;
    let pairs = sheet_ranges::load_cabinet_advert_nm_rows(
        &client, src_id, src_sheet, cab_range, id_range, nm_range,
    )
    .await?;
    ctx.log.info(format!("Пар кабинет/кампания: {}", pairs.len()));
    if pairs.is_empty() {
        return Err("Нет пар кабинет/кампания в источнике".into());
    }

    let allowed: HashSet<String> = common::resolve_cabinets(ctx)
        .into_iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut by_cab: HashMap<String, Vec<(i64, Option<i64>)>> = HashMap::new();
    for (cab, adv, nm) in pairs {
        if !allowed.is_empty() && !allowed.contains(&cab.to_ascii_lowercase()) {
            continue;
        }
        by_cab.entry(cab).or_default().push((adv, nm));
    }
    if by_cab.is_empty() {
        return Err("Нет кампаний для выбранных кабинетов".into());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let cab_order: Vec<String> = by_cab.keys().cloned().collect();
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let items = by_cab.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let fields = fields.clone();
            let from_s = from_s.clone();
            let to_s = to_s.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut rows = Vec::new();
                for chunk in items.chunks(BATCH) {
                    let body_items: Vec<Value> = chunk
                        .iter()
                        .map(|(adv, nm)| {
                            let mut o = json!({ "advert_id": adv });
                            if let Some(n) = nm {
                                o["nm_ids"] = json!([n]);
                            }
                            o
                        })
                        .collect();
                    let body = json!({
                        "from": from_s,
                        "to": to_s,
                        "items": body_items,
                    });
                    let resp = fetch(ctx, &cabinet, token.trim(), &body).await?;
                    append_stats_rows(&mut rows, &fields, &cabinet, &resp);
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
            STATS_URL,
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
