//! Цены по nmList (`prices_by_nm`) — POST filter + Second sheet.

use crate::core::rate_profiles::prices_by_nm as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name, parse_i64_cell};
use crate::scripts::wb_prices::fields::{build_rows, default_fields_by_nm};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const PRICES_URL: &str = "https://discounts-prices-api.wildberries.ru/api/v2/list/goods/filter";
const BATCH: usize = 1000;

pub struct PricesByNmScript;

impl Script for PricesByNmScript {
    fn key(&self) -> &'static str {
        "prices_by_nm"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.prices_by_nm")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields_by_nm()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> Option<&'a str> {
    ctx.params.get(key).map(|s| s.as_str()).filter(|s| !s.trim().is_empty())
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Старт");

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
    if cab_range.is_empty() || nm_range.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов и nmID в источнике".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(
        &client,
        src_id,
        src_sheet,
        &[cab_range, nm_range],
    )
    .await?;
    let cabinets = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    let n = cabinets.len().max(nms.len());

    let mut per_cab: HashMap<String, HashSet<i64>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    for i in 0..n {
        let cab_raw = cabinets.get(i).map(|s| s.as_str()).unwrap_or("").trim();
        let nm_raw = nms.get(i).map(|s| s.as_str()).unwrap_or("");
        if cab_raw.is_empty() {
            continue;
        }
        let Some(nm) = parse_i64_cell(nm_raw).filter(|&x| x > 0) else {
            continue;
        };
        let key = normalize_cabinet_name(cab_raw).to_ascii_lowercase();
        per_cab.entry(key.clone()).or_default().insert(nm);
        labels.entry(key).or_insert_with(|| cab_raw.to_string());
    }
    if per_cab.is_empty() {
        return Ok(common::empty_data_output());
    }

    let fields = common::resolve_fields(ctx, default_fields_by_nm());
    let wb_tokens = common::load_wb_tokens()?;
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;

    for (norm, nm_set) in &per_cab {
        let label = labels.get(norm).cloned().unwrap_or_else(|| norm.clone());
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &label);
        if token.trim().is_empty() {
            ctx.log
                .warn(format!("Кабинет «{label}»: нет API_KEY, пропуск"));
            continue;
        }
        let mut nm_list: Vec<i64> = nm_set.iter().copied().collect();
        nm_list.sort_unstable();
        let goods = fetch_by_nm(ctx, &label, token.trim(), &nm_list).await?;
        ctx.log
            .info(format!("Кабинет «{label}»: {} позиций", goods.len()));
        total += goods.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&label, &goods, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} позиций на лист «{dest_sheet}»"),
    })
}

async fn fetch_by_nm(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    nm_list: &[i64],
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    for chunk in nm_list.chunks(BATCH) {
        let body = json!({ "nmList": chunk });
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                PRICES_URL,
                Auth::Bearer(token),
                &body,
            )
            .await?;
        let goods = v
            .pointer("/data/listGoods")
            .and_then(|g| g.as_array())
            .cloned()
            .unwrap_or_default();
        all.extend(goods);
    }
    Ok(all)
}
