//! `nm_report_downloads_list`.

use super::api::{self, list_cell};
use super::api::list_fields;
use crate::core::rate_profiles::nm_report::list as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;

pub struct NmReportDownloadsListScript;

impl Script for NmReportDownloadsListScript {
    fn key(&self) -> &'static str {
        "nm_report_downloads_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.nm_report_downloads_list")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        list_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Nm-report list: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, list_fields());
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
                let items = api::list_downloads(
                    &ctx.request,
                    &cabinet,
                    token.trim(),
                    rate::SCOPE,
                    &profile,
                    &[],
                )
                .await
                .unwrap_or_else(|e| {
                    ctx.log.warn(e);
                    Vec::new()
                });
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} отчётов", items.len()));
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        for it in items {
            total += 1;
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(list_cell(f.name.as_str(), &cabinet, &it));
            }
            rows.push(row);
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} записей списка nm-report на «{sheet_name}»"),
    })
}
