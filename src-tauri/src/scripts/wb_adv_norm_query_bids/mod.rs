//! Ставки поисковых кластеров (чтение): `POST /adv/v0/normquery/get-bids`.

use crate::core::rate_profiles::wb_adv_norm_query_bids as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

const BIDS_URL: &str = "https://advert-api.wildberries.ru/adv/v0/normquery/get-bids";
const BATCH: usize = 100;

pub struct AdvNormQueryBidsScript;

impl Script for AdvNormQueryBidsScript {
    fn key(&self) -> &'static str {
        "wb_adv_norm_query_bids"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_adv_norm_query_bids")
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
    ctx.log.info("Скрипт «Ставки поисковых кластеров»: старт");

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
    if cab_range.is_empty() || id_range.is_empty() || nm_range.is_empty() {
        return Err(
            "Укажите диапазоны кабинетов, ID кампаний и nmID.".into(),
        );
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let client = common::google_sheets_client(ctx).await?;
    let pairs = sheet_ranges::load_cabinet_advert_nm_required(
        &client, src_id, src_sheet, cab_range, id_range, nm_range,
    )
    .await?;
    ctx.log
        .info(format!("Троек кабинет/кампания/nm: {}", pairs.len()));
    if pairs.is_empty() {
        return Err("Нет троек кабинет/кампания/nm в источнике".into());
    }

    let allowed: HashSet<String> = common::resolve_cabinets(ctx)
        .into_iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let mut by_cab: HashMap<String, Vec<(i64, i64)>> = HashMap::new();
    for (cab, adv, nm) in pairs {
        if !allowed.is_empty() && !allowed.contains(&cab.to_ascii_lowercase()) {
            continue;
        }
        by_cab.entry(cab).or_default().push((adv, nm));
    }
    if by_cab.is_empty() {
        return Err("Нет строк для выбранных кабинетов".into());
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
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut rows = Vec::new();
                for chunk in items.chunks(BATCH) {
                    let body = json!({
                        "items": chunk.iter().map(|(adv, nm)| json!({
                            "advert_id": adv,
                            "nm_id": nm,
                        })).collect::<Vec<_>>()
                    });
                    let resp = fetch(ctx, &cabinet, token.trim(), &body).await?;
                    append_bid_rows(&mut rows, &fields, &cabinet, &resp);
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
            BIDS_URL,
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
        field("advertId", true),
        field("nmId", true),
        field("normQuery", true),
        field("bid", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_norm_query_bids.{name}")),
        selected,
    }
}

pub fn append_bid_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let bids = root
        .get("bids")
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();
    for b in &bids {
        let advert_id = first_num(b, &["advert_id", "advertId"]);
        let nm_id = first_num(b, &["nm_id", "nmId"]);
        let norm_query = first_str(b, &["norm_query", "normQuery"]);
        let bid = first_num(b, &["bid"]);
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "advertId" => advert_id.clone(),
                    "nmId" => nm_id.clone(),
                    "normQuery" => norm_query.clone(),
                    "bid" => bid.clone(),
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
