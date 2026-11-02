//! `prices_set` — точечная установка цены и/или скидки.

use super::upload::{upload_batches, PriceItem};
use crate::core::rate_profiles::prices_set as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name, parse_i64_cell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct PricesSetScript;

impl Script for PricesSetScript {
    fn key(&self) -> &'static str {
        "prices_set"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.prices_set")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
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
    ctx.log.info("Старт");
    let (sheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let nm_r = param(ctx, "nm_id_source_range").unwrap_or("");
    let price_r = param(ctx, "price_set_price_range").unwrap_or("");
    let disc_r = param(ctx, "price_set_discount_range").unwrap_or("");
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    if nm_r.is_empty() || cab_r.is_empty() || (price_r.is_empty() && disc_r.is_empty()) {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите nmID, кабинет и хотя бы один диапазон цены/скидки".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let mut range_list = vec![cab_r, nm_r];
    if !price_r.is_empty() {
        range_list.push(price_r);
    }
    if !disc_r.is_empty() {
        range_list.push(disc_r);
    }
    let cols = sheet_ranges::read_first_cols(&client, &sheet_id, &sheet_name, &range_list).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    let mut idx = 2usize;
    let prices = if !price_r.is_empty() {
        let c = cols.get(idx).cloned().unwrap_or_default();
        idx += 1;
        Some(c)
    } else {
        None
    };
    let discs = if !disc_r.is_empty() {
        cols.get(idx).cloned()
    } else {
        None
    };

    let n = cabs.len().max(nms.len());
    let mut by_cab: HashMap<String, Vec<PriceItem>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();

    for i in 0..n {
        let cab = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let nm = nms.get(i).map(|s| s.as_str()).unwrap_or("");
        if cab.is_empty() {
            continue;
        }
        let Some(nm_id) = parse_i64_cell(nm).filter(|&x| x > 0) else {
            continue;
        };
        let price = prices
            .as_ref()
            .and_then(|p| p.get(i))
            .and_then(|s| parse_i64_loose(s));
        let discount = discs
            .as_ref()
            .and_then(|p| p.get(i))
            .and_then(|s| parse_i64_loose(s));
        if price.is_none() && discount.is_none() {
            continue;
        }
        let key = normalize_cabinet_name(cab).to_ascii_lowercase();
        labels.entry(key.clone()).or_insert_with(|| cab.to_string());
        by_cab.entry(key).or_default().push(PriceItem {
            nm_id,
            price,
            discount,
        });
    }

    if by_cab.is_empty() {
        return Ok(common::empty_data_output());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut total = 0usize;
    for (norm, items) in &by_cab {
        let label = labels.get(norm).cloned().unwrap_or_else(|| norm.clone());
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &label);
        if token.trim().is_empty() {
            ctx.log.warn(format!("Кабинет «{label}»: нет токена"));
            continue;
        }
        match upload_batches(
            &ctx.request,
            &label,
            token.trim(),
            rate::SCOPE,
            &profile,
            items,
            false,
            5,
        )
        .await
        {
            Ok(n) => {
                ctx.log.info(format!("Кабинет «{label}»: отправлено {n}"));
                total += n;
            }
            Err(e) => ctx.log.warn(e),
        }
    }

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Отправлено позиций: {total}"),
    })
}

fn parse_i64_loose(raw: &str) -> Option<i64> {
    let s = raw.trim().replace('\u{00a0}', "").replace(' ', "").replace(',', ".");
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok().map(|f| f.round() as i64)
}
