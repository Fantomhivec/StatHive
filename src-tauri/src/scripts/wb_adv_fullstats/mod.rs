//! Статистика РК: `wb_adv_fullstats` — `GET /adv/v3/fullstats` (ids≤50, период ≤31д).

pub mod fields;

use crate::core::rate_profiles::wb_adv_fullstats as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use crate::scripts::sheet_ranges;
use crate::scripts::wb_adv_upd::fields::build_chunks;
use self::fields::{append_fullstats_rows, default_fields};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const FULLSTATS_URL: &str = "https://advert-api.wildberries.ru/adv/v3/fullstats";
const CHUNK: usize = 50;

pub struct AdvFullstatsScript;

impl Script for AdvFullstatsScript {
    fn key(&self) -> &'static str {
        "wb_adv_fullstats"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_fullstats")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_fullstats(ctx).await })
    }
}

async fn run_fullstats(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Статистика рекламных кампаний»: старт");

    let (from_dt, to_dt) = dates::resolve_period(ctx, 14)?;
    let chunks = build_chunks(from_dt, to_dt, 31);
    ctx.log.info(format!(
        "Период: {from_dt}..{to_dt}; чанков дат: {}",
        chunks.len()
    ));

    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let src_id = param(ctx, "second_spreadsheet_id")
        .or_else(|| param(ctx, "source_spreadsheet_id"))
        .unwrap_or_else(|| dest_id.clone());
    let src_sheet = param(ctx, "second_sheet_name")
        .or_else(|| param(ctx, "source_sheet_name"))
        .unwrap_or_else(|| "Кампании".into());
    let id_range = param(ctx, "id_adv_campaign_source_range").unwrap_or_default();
    let cab_range = param(ctx, "cabinets_source_range").unwrap_or_default();
    if id_range.trim().is_empty() || cab_range.trim().is_empty() {
        return Err(
            "Укажите диапазоны ID кампаний и кабинетов (id_adv_campaign_source_range, cabinets_source_range)."
                .into(),
        );
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }
    let client = common::google_sheets_client(ctx).await?;
    let pairs = sheet_ranges::load_cabinet_id_pairs(
        &client,
        src_id.trim(),
        src_sheet.trim(),
        &cab_range,
        &id_range,
    )
    .await?;
    ctx.log
        .info(format!("Прочитано пар кабинет/ID: {}", pairs.len()));
    if pairs.is_empty() {
        return Err("Нет пар [кабинет, ID кампании] в источнике".into());
    }

    let allowed: HashSet<String> = common::resolve_cabinets(ctx)
        .into_iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut by_cabinet: HashMap<String, Vec<String>> = HashMap::new();
    for (cab, id) in pairs {
        if !allowed.is_empty() && !allowed.contains(&cab.to_ascii_lowercase()) {
            continue;
        }
        by_cabinet.entry(cab).or_default().push(id);
    }
    if by_cabinet.is_empty() {
        return Err("Нет кампаний для выбранных кабинетов".into());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let cab_order: Vec<String> = by_cabinet.keys().cloned().collect();
    let date_chunks = chunks.clone();
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let mut ids = by_cabinet.get(&cabinet).cloned().unwrap_or_default();
            ids.sort();
            ids.dedup();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let fields = fields.clone();
            let date_chunks = date_chunks.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut rows = Vec::new();
                for (d_from, d_to) in &date_chunks {
                    let begin = d_from.format("%Y-%m-%d").to_string();
                    let end = d_to.format("%Y-%m-%d").to_string();
                    for id_chunk in ids.chunks(CHUNK) {
                        let items =
                            fetch_fullstats(ctx, &cabinet, token.trim(), id_chunk, &begin, &end)
                                .await?;
                        for item in &items {
                            append_fullstats_rows(&mut rows, &fields, &cabinet, item);
                        }
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
    for (_cabinet, rows) in parts {
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

async fn fetch_fullstats(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    ids: &[String],
    begin: &str,
    end: &str,
) -> Result<Vec<Value>, String> {
    let ids_param = ids.join(",");
    let url = format!("{FULLSTATS_URL}?ids={ids_param}&beginDate={begin}&endDate={end}");
    ctx.log.info(format!(
        "GET adv/v3/fullstats ids={} {begin}..{end} (кабинет «{cabinet}»)",
        ids.len()
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
    Ok(match v {
        Value::Array(a) => a,
        other => other
            .as_array()
            .cloned()
            .or_else(|| {
                other
                    .get("data")
                    .and_then(|d| d.as_array())
                    .cloned()
            })
            .unwrap_or_default(),
    })
}

fn param(ctx: &ScriptContext, key: &str) -> Option<String> {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
