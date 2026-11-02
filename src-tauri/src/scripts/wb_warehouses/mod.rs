pub mod fields;

use crate::core::rate_profiles::warehouses_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_rows, default_fields, merge_cabinet_rows};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const WH_URL: &str = "https://marketplace-api.wildberries.ru/api/v3/warehouses";

/// Список складов продавца (`warehouses_list`) → Google Sheets.
pub struct WarehousesScript;

impl Script for WarehousesScript {
    fn key(&self) -> &'static str {
        "warehouses_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouses_list")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_warehouses(ctx).await })
    }
}

async fn run_warehouses(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Список складов продавца»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let list = fetch_warehouses(ctx, &cabinet, token.trim()).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} складов", list.len()));
                Ok((cabinet, list))
            }
        },
    )
    .await?;

    let mut body_rows: Vec<Vec<String>> = Vec::new();
    let mut header: Option<Vec<String>> = None;
    let mut total = 0usize;
    for (cabinet, list) in parts {
        if list.is_empty() {
            continue;
        }
        total += list.len();
        let mut rows = build_rows(&cabinet, &list, &fields);
        if header.is_none() && !rows.is_empty() {
            header = Some(rows.remove(0));
        } else if !rows.is_empty() {
            rows.remove(0);
        }
        body_rows.extend(rows);
    }

    let Some(hdr) = header else {
        return Ok(common::empty_data_output());
    };

    let cab_col = fields
        .iter()
        .filter(|f| f.selected)
        .position(|f| f.name.eq_ignore_ascii_case("cabinet"))
        .unwrap_or(0);
    let all_rows = merge_cabinet_rows(hdr, body_rows, cab_col, &cabinets);

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано складов (до сжатия): {total} → лист «{sheet_name}»"),
    })
}

async fn fetch_warehouses(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    ctx.log.info("GET marketplace /api/v3/warehouses");
    let profile = rate::profile();
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            WH_URL,
            Auth::Raw(token),
        )
        .await?;
    Ok(match v {
        Value::Array(a) => a,
        _ => Vec::new(),
    })
}
