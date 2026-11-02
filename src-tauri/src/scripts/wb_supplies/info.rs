//! `supply_info` — detail by supply id (Second + ranges).

use super::api::{self, info_fields, row_from};
use crate::core::rate_profiles::supply_info as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct SupplyInfoScript;

impl Script for SupplyInfoScript {
    fn key(&self) -> &'static str {
        "supply_info"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.supply_info")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        info_fields()
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
    ctx.log.info("Скрипт «Инфо поставки»: старт");
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
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов и ID поставок (как nmID)".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let pairs =
        sheet_ranges::load_cabinet_id_pairs(&client, src_id, src_sheet, cab_r, id_r).await?;
    let mut by_cab: HashMap<String, Vec<String>> = HashMap::new();
    for (cab, id) in pairs {
        let id = id.trim().to_string();
        if id.is_empty() {
            continue;
        }
        by_cab.entry(cab).or_default().push(id);
    }
    if by_cab.is_empty() {
        return Ok(common::empty_data_output());
    }

    let fields = common::resolve_fields(ctx, info_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut all_rows = vec![common::selected_titles(&fields)];
    let mut total = 0usize;

    for (cabinet, ids) in &by_cab {
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, cabinet);
        if token.trim().is_empty() {
            ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
            continue;
        }
        for sid in ids {
            match api::supply_info(
                &ctx.request,
                cabinet,
                token.trim(),
                rate::SCOPE,
                &profile,
                sid,
            )
            .await
            {
                Ok(it) => {
                    all_rows.push(row_from(cabinet, &it, &selected, &[]));
                    total += 1;
                }
                Err(e) => ctx.log.warn(format!("{cabinet}/{sid}: {e}")),
            }
        }
    }

    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано карточек поставок: {total}"),
    })
}
