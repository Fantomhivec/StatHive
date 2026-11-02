//! `paid_storage_create_task`.

use super::api::{self, create_row, split_intervals};
use super::api::create_task_fields;
use crate::core::rate_profiles::paid_storage_create_task as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;

pub struct PaidStorageCreateTaskScript;

impl Script for PaidStorageCreateTaskScript {
    fn key(&self) -> &'static str {
        "paid_storage_create_task"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.paid_storage_create_task")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        create_task_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 7)?;
    let intervals = split_intervals(start, end, 8);
    ctx.log.info(format!(
        "Период {}..{}, интервалов: {}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d"),
        intervals.len()
    ));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, create_task_fields());
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let intervals = intervals.clone();
            let profile = profile.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<(chrono::NaiveDate, chrono::NaiveDate, String)>::new()));
                }
                let mut created = Vec::new();
                for (from, to) in &intervals {
                    match api::create_task(
                        &ctx.request,
                        &cabinet,
                        token.trim(),
                        rate::SCOPE,
                        &profile,
                        *from,
                        *to,
                    )
                    .await
                    {
                        Ok(task_id) => {
                            ctx.log.info(format!(
                                "Кабинет «{cabinet}»: taskId={task_id} ({from}..{to})"
                            ));
                            created.push((*from, *to, task_id));
                        }
                        Err(e) => ctx.log.warn(e),
                    }
                }
                Ok((cabinet, created))
            }
        },
    )
    .await?;

    let mut all_rows = vec![common::selected_titles(&fields)];
    let mut total = 0usize;
    for (cabinet, created) in parts {
        for (from, to, task_id) in created {
            all_rows.push(create_row(&cabinet, from, to, &task_id));
            total += 1;
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Создано заданий: {total}"),
    })
}
