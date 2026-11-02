//! Инфо РК: `wb_adv_campaigns_stats_new` — advertId → nmId (`GET /api/advert/v2/adverts`).

pub mod fields;

use crate::core::rate_profiles::wb_adv_campaigns_stats_new as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use self::fields::{append_advert_rows, default_fields};
use crate::scripts::common;
use crate::scripts::sheet_ranges;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const ADVERTS_URL: &str = "https://advert-api.wildberries.ru/api/advert/v2/adverts";
const CHUNK: usize = 50;

/// Связка ID кампании ↔ артикул WB → лист «Инфо».
pub struct AdvCampaignsStatsNewScript;

impl Script for AdvCampaignsStatsNewScript {
    fn key(&self) -> &'static str {
        "wb_adv_campaigns_stats_new"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_campaigns_stats_new")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_stats(ctx).await })
    }
}

async fn run_stats(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Инфо рекламных кампаний»: старт");

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

    ctx.log.info(format!(
        "Источник: {src_id} / «{src_sheet}»; ID={id_range}, кабинеты={cab_range}"
    ));

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
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let mut ids = by_cabinet.get(&cabinet).cloned().unwrap_or_default();
            ids.sort();
            ids.dedup();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let fields = fields.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: {} уникальных кампаний",
                    ids.len()
                ));
                let mut rows = Vec::new();
                for chunk in ids.chunks(CHUNK) {
                    let adverts = fetch_adverts(ctx, &cabinet, token.trim(), chunk).await?;
                    for adv in &adverts {
                        append_advert_rows(&mut rows, &fields, &cabinet, adv);
                    }
                }
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: добавлено строк {}",
                    rows.len()
                ));
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

async fn fetch_adverts(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    ids: &[String],
) -> Result<Vec<Value>, String> {
    let ids_param = ids.join(",");
    let url = format!("{ADVERTS_URL}?ids={ids_param}");
    ctx.log.info(format!(
        "GET advert/v2/adverts ids={} (кабинет «{cabinet}»)",
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
    Ok(v.get("adverts")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default())
}

fn param(ctx: &ScriptContext, key: &str) -> Option<String> {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
