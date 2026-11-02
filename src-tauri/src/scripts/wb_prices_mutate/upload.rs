//! `prices_upload` — массовая установка цен из листа.


// --- upload_api (merged) ---
use crate::core::request_sender::{Auth, Limit, RequestSender};
use crate::core::rate_limit::RateLimitProfile;
use serde_json::{json, Value};

const URL: &str = "https://discounts-prices-api.wildberries.ru/api/v2/upload/task";
pub const BATCH: usize = 1000;

#[derive(Clone)]
pub struct PriceItem {
    pub nm_id: i64,
    pub price: Option<i64>,
    pub discount: Option<i64>,
}

pub async fn upload_batches(
    request: &RequestSender,
    cabinet: &str,
    token: &str,
    scope: &str,
    profile: &RateLimitProfile,
    items: &[PriceItem],
    require_both: bool,
    max_retries: u32,
) -> Result<usize, String> {
    let mut sent = 0usize;
    for chunk in items.chunks(BATCH) {
        let data: Vec<Value> = chunk
            .iter()
            .filter_map(|it| {
                if require_both {
                    Some(json!({
                        "nmID": it.nm_id,
                        "price": it.price.unwrap_or(0),
                        "discount": it.discount.unwrap_or(0),
                    }))
                } else {
                    let mut obj = serde_json::Map::new();
                    obj.insert("nmID".into(), json!(it.nm_id));
                    if let Some(p) = it.price {
                        obj.insert("price".into(), json!(p));
                    }
                    if let Some(d) = it.discount {
                        obj.insert("discount".into(), json!(d));
                    }
                    if obj.len() == 1 {
                        return None;
                    }
                    Some(Value::Object(obj))
                }
            })
            .collect();
        if data.is_empty() {
            continue;
        }
        let body = json!({ "data": data });
        let mut ok = false;
        let mut last_err = String::new();
        for _ in 0..max_retries {
            match request
                .post_json(
                    Limit {
                        cabinet,
                        scope,
                        profile,
                    },
                    URL,
                    Auth::Bearer(token),
                    &body,
                )
                .await
            {
                Ok(_) => {
                    ok = true;
                    break;
                }
                Err(e) => {
                    last_err = e;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        if !ok {
            return Err(format!("Кабинет {cabinet}: upload failed: {last_err}"));
        }
        sent += data.len();
    }
    Ok(sent)
}

use crate::core::rate_profiles::prices_upload as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name, parse_i64_cell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

pub struct PricesUploadScript;

impl Script for PricesUploadScript {
    fn key(&self) -> &'static str {
        "prices_upload"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.prices_upload")
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
    let price_r = param(ctx, "prices_price_range").unwrap_or("");
    let disc_r = param(ctx, "prices_discount_range").unwrap_or("");
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    if nm_r.is_empty() || price_r.is_empty() || disc_r.is_empty() || cab_r.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны nmID, цены, скидки и кабинетов".into(),
        });
    }
    let exclude_r = param(ctx, "prices_exclude_range").unwrap_or("");
    let raise = param(ctx, "prices_raise_percent")
        .and_then(|s| s.replace(',', ".").parse::<f64>().ok())
        .unwrap_or(0.0);

    let client = common::google_sheets_client(ctx).await?;

    let mut ranges = vec![cab_r, nm_r, price_r, disc_r];
    if !exclude_r.is_empty() {
        ranges.push(exclude_r);
    }
    let cols = sheet_ranges::read_first_cols(&client, &sheet_id, &sheet_name, &ranges).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    let prices = cols.get(2).cloned().unwrap_or_default();
    let discs = cols.get(3).cloned().unwrap_or_default();
    let exclude: HashSet<i64> = cols
        .get(4)
        .into_iter()
        .flatten()
        .filter_map(|s| parse_i64_cell(s))
        .collect();

    let n = cabs.len().max(nms.len()).max(prices.len()).max(discs.len());
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
        if exclude.contains(&nm_id) {
            continue;
        }
        let price = parse_price(prices.get(i).map(|s| s.as_str()).unwrap_or(""));
        let discount = parse_discount(discs.get(i).map(|s| s.as_str()).unwrap_or("")).unwrap_or(0);
        let Some(mut price) = price else { continue };
        if raise != 0.0 {
            let final_now = price as f64 * (100.0 - discount as f64) / 100.0;
            let target = final_now * (100.0 + raise) / 100.0;
            let denom = (100.0 - discount as f64).max(1.0);
            price = (target * 100.0 / denom).round() as i64;
        }
        let key = normalize_cabinet_name(cab).to_ascii_lowercase();
        labels.entry(key.clone()).or_insert_with(|| cab.to_string());
        by_cab.entry(key).or_default().push(PriceItem {
            nm_id,
            price: Some(price),
            discount: Some(discount),
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
        let n = upload_batches(
            &ctx.request,
            &label,
            token.trim(),
            rate::SCOPE,
            &profile,
            items,
            true,
            3,
        )
        .await?;
        ctx.log.info(format!("Кабинет «{label}»: отправлено {n}"));
        total += n;
    }

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Отправлено позиций цен: {total}"),
    })
}

fn parse_price(raw: &str) -> Option<i64> {
    let s = raw.trim().replace('\u{00a0}', "").replace(' ', "").replace(',', ".");
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok().map(|f| f.round() as i64)
}

fn parse_discount(raw: &str) -> Option<i64> {
    let s = raw.trim().replace('%', "").replace(',', ".");
    if s.is_empty() {
        return None;
    }
    let v = s.parse::<f64>().ok()?;
    let pct = if (0.0..1.0).contains(&v) && v != 0.0 {
        v * 100.0
    } else {
        v
    };
    Some(pct.round().clamp(0.0, 100.0) as i64)
}
