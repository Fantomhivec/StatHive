//! `nm_report_downloads` — create DETAIL_HISTORY_REPORT.

use super::api::{self, create_row};
use super::api::create_fields;
use crate::core::rate_profiles::nm_report::create as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

pub struct NmReportDownloadsScript;

impl Script for NmReportDownloadsScript {
    fn key(&self) -> &'static str {
        "nm_report_downloads"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.nm_report_downloads")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        create_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Nm-report create: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 14)?;
    if (end - start).num_days() > 365 {
        return Err("Максимум 365 дней для DETAIL_HISTORY_REPORT.".into());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, create_fields());
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
                    return Ok((cabinet, None::<String>));
                }
                let id = Uuid::new_v4().to_string();
                match api::create_detail_history(
                    &ctx.request,
                    &cabinet,
                    token.trim(),
                    rate::SCOPE,
                    &profile,
                    &id,
                    start,
                    end,
                )
                .await
                {
                    Ok(()) => {
                        ctx.log
                            .info(format!("Кабинет «{cabinet}»: downloadId={id}"));
                        Ok((cabinet, Some(id)))
                    }
                    Err(e) => {
                        ctx.log.warn(e);
                        Ok((cabinet, None))
                    }
                }
            }
        },
    )
    .await?;

    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, id) in parts {
        let Some(id) = id else { continue };
        total += 1;
        let full = create_row(&cabinet, start, end, &id, "DETAIL_HISTORY_REPORT");
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            let idx = match f.name.as_str() {
                "cabinet" => 0,
                "downloadId" => 1,
                "dateFrom" => 2,
                "dateTo" => 3,
                "reportType" => 4,
                _ => continue,
            };
            row.push(full.get(idx).cloned().unwrap_or_default());
        }
        rows.push(row);
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Создано {total} заданий nm-report на лист «{sheet_name}»"),
    })
}
