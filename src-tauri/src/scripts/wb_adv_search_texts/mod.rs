//! Поисковые тексты по товару: `POST /api/v2/search-report/product/search-texts`.

use crate::core::rate_profiles::wb_adv_search_texts as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use crate::scripts::sheet_ranges;
use chrono::Duration;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

const URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/v2/search-report/product/search-texts";
const NM_BATCH: usize = 20;

pub struct AdvSearchTextsScript;

impl Script for AdvSearchTextsScript {
    fn key(&self) -> &'static str {
        "wb_adv_search_texts"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_search_texts")
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
    ctx.log.info("Скрипт «Поисковые тексты по товарам»: старт");
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
    if cab_range.is_empty() || nm_range.is_empty() {
        return Err("Укажите диапазоны кабинетов и nmID.".into());
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let client = common::google_sheets_client(ctx).await?;
    let by_cab =
        sheet_ranges::load_cabinet_nm_map(&client, src_id, src_sheet, cab_range, nm_range).await?;
    ctx.log.info(format!("Кабинетов с nm: {}", by_cab.len()));
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
        return Err("Нет nm для выбранных кабинетов".into());
    }

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
                for chunk in nms.chunks(NM_BATCH) {
                    let body = json!({
                        "nmIds": chunk,
                        "currentPeriod": { "start": cur_s, "end": cur_e },
                        "pastPeriod": { "start": past_s_s, "end": past_e_s },
                        "topOrderBy": "orders",
                        "includeSearchTexts": true,
                        "includeSubstitutedSKUs": true,
                        "orderBy": { "field": "orders", "mode": "desc" },
                        "limit": 30
                    });
                    let resp = fetch(ctx, &cabinet, token.trim(), &body).await?;
                    // API may return per-nm nested or flat; attach first nm of chunk if flat.
                    if let Some(arr) = resp.get("data").and_then(|d| d.as_array()) {
                        for (i, item) in arr.iter().enumerate() {
                            let nm = chunk.get(i).copied().unwrap_or(chunk[0]);
                            append_text_rows(&mut rows, &fields, &cabinet, nm, item);
                        }
                    } else {
                        for &nm in chunk {
                            append_text_rows(&mut rows, &fields, &cabinet, nm, &resp);
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

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("text", true),
        field("frequency", true),
        field("avgPosition", true),
        field("openCard", true),
        field("addToCart", true),
        field("orders", true),
        field("visibility", true),
        field("openToCart", true),
        field("cartToOrder", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_search_texts.{name}")),
        selected,
    }
}

pub fn append_text_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    nm_id: i64,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let data = root.get("data").unwrap_or(root);
    let items = data
        .get("items")
        .or_else(|| data.get("searchTexts"))
        .or_else(|| data.as_array().map(|_| data))
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    for it in &items {
        let text = first_str(it, &["text", "searchText", "name"]);
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "nmId" => nm_id.to_string(),
                    "text" => text.clone(),
                    "frequency" => first_num(it, &["frequency", "freq"]),
                    "avgPosition" => first_num(it, &["avgPosition", "avg_position"]),
                    "openCard" => first_num(it, &["openCard", "open_card"]),
                    "addToCart" => first_num(it, &["addToCart", "add_to_cart"]),
                    "orders" => first_num(it, &["orders"]),
                    "visibility" => first_num(it, &["visibility"]),
                    "openToCart" => first_num(it, &["openToCart", "open_to_cart"]),
                    "cartToOrder" => first_num(it, &["cartToOrder", "cart_to_order"]),
                    _ => String::new(),
                })
                .collect(),
        );
    }
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(Value::String(s)) = v.get(*k) {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    String::new()
}

fn first_num(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match v.get(*k) {
            Some(Value::Number(n)) => return n.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() => return s.clone(),
            _ => {}
        }
    }
    String::new()
}
