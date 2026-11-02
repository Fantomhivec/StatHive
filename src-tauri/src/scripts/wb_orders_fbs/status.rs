//! `orders_fbs_status` — POST marketplace `/api/v3/orders/status`.

use super::fields::{build_status_rows, status_fields};
use crate::core::rate_profiles::orders_fbs_status as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, parse_i64_cell};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://marketplace-api.wildberries.ru/api/v3/orders/status";
const BATCH: usize = 1000;

pub struct OrdersFbsStatusScript;

impl Script for OrdersFbsStatusScript {
    fn key(&self) -> &'static str {
        "orders_fbs_status"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.orders_fbs_status")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        status_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> Option<&'a str> {
    ctx.params
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Статусы заказов FBS»: старт");
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
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    let id_r = param(ctx, "nm_id_source_range").unwrap_or("");
    if cab_r.is_empty() || id_r.is_empty() {
        return Err("Укажите диапазоны кабинетов и ID заказов (как nmID).".into());
    }

    let fields = common::resolve_fields(ctx, status_fields());
    let client = common::google_sheets_client(ctx).await?;
    let pairs =
        sheet_ranges::load_cabinet_id_pairs(&client, src_id, src_sheet, cab_r, id_r).await?;
    let mut by_cab: HashMap<String, Vec<i64>> = HashMap::new();
    for (cab, id_s) in pairs {
        if let Some(id) = parse_i64_cell(&id_s) {
            by_cab.entry(cab).or_default().push(id);
        }
    }
    if by_cab.is_empty() {
        return Ok(common::empty_data_output());
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
    if cab_order.is_empty() {
        return Err("Нет ID заказов для выбранных кабинетов".into());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let ids = by_cab.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let items = fetch_statuses(ctx, &cabinet, token.trim(), &ids).await?;
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: статусов {}",
                    items.len()
                ));
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, items) in parts {
        if items.is_empty() {
            continue;
        }
        total += items.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_status_rows(&cabinet, &items, &fields),
        );
    }
    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} статусов заказов на лист «{dest_sheet}»"),
    })
}

async fn fetch_statuses(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    ids: &[i64],
) -> Result<Vec<Value>, String> {
    let profile = rate::profile();
    let mut all = Vec::new();
    for chunk in ids.chunks(BATCH) {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let body = json!({ "orders": chunk });
        let mut v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                URL,
                Auth::Raw(token),
                &body,
            )
            .await?;
        let orders = v
            .get_mut("orders")
            .and_then(|x| x.as_array_mut())
            .map(std::mem::take)
            .unwrap_or_default();
        all.extend(orders);
    }
    Ok(all)
}
