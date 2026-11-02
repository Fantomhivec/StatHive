//! `supplies_warehouses` — FBO acceptance warehouses справочник.

use super::api::{self, row_from, warehouses_fields};
use crate::core::rate_profiles::supplies_warehouses as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;

pub struct SuppliesWarehousesScript;

impl Script for SuppliesWarehousesScript {
    fn key(&self) -> &'static str {
        "supplies_warehouses"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.supplies_warehouses")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        warehouses_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Склады поставок FBO»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, warehouses_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::new()));
                }
                let list = api::fbo_warehouses(
                    &ctx.request,
                    &cabinet,
                    token.trim(),
                    rate::SCOPE,
                    &profile,
                )
                .await
                .unwrap_or_else(|e| {
                    ctx.log.warn(e);
                    Vec::new()
                });
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} складов поставок", list.len()));
                Ok((cabinet, list))
            }
        },
    )
    .await?;

    let mut all_rows = vec![common::selected_titles(&fields)];
    let mut total = 0usize;
    for (cabinet, list) in parts {
        for it in &list {
            all_rows.push(row_from(&cabinet, it, &selected, &[]));
            total += 1;
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано складов поставок: {total}"),
    })
}
