//! Маркировка: `GET /api/v1/analytics/goods-labeling` (период ≤31д).

pub mod fields;

use crate::core::rate_profiles::goods_labeling as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::dates;
use crate::scripts::wb_adv_upd::fields::build_chunks;
use self::fields::{append_rows, default_fields};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/v1/analytics/goods-labeling";

pub struct GoodsLabelingScript;

impl Script for GoodsLabelingScript {
    fn key(&self) -> &'static str {
        "goods_labeling"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.goods_labeling")
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
    ctx.log.info("Скрипт «Удержания за маркировку»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (from_dt, to_dt) = dates::resolve_period(ctx, 14)?;
    let chunks = build_chunks(from_dt, to_dt, 31);

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Ok(common::empty_data_output());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let date_chunks = chunks.clone();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
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
                    let url = format!(
                        "{URL}?dateFrom={}&dateTo={}",
                        d_from.format("%Y-%m-%d"),
                        d_to.format("%Y-%m-%d")
                    );
                    let resp = fetch(ctx, &cabinet, token.trim(), &url).await?;
                    append_rows(&mut rows, &fields, &cabinet, &resp);
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
    let client = common::google_sheets_client(ctx).await?;
    common::write_sheet_rows_with(ctx, &client, spreadsheet_id.trim(), sheet_name.trim(), all_rows)
        .await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк на лист «{sheet_name}»"),
    })
}

async fn fetch(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    url: &str,
) -> Result<Value, String> {
    let profile = rate::profile();
    ctx.request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            url,
            Auth::Raw(token),
        )
        .await
}
