//! `paid_storage_auto` — create → wait → download.

use super::api::{self, is_done_status, is_failed_status, report_cell, split_intervals};
use super::api::default_fields;
use crate::core::rate_profiles::paid_storage_auto as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;
use tokio::time::{sleep, Duration};

pub struct PaidStorageAutoScript;

impl Script for PaidStorageAutoScript {
    fn key(&self) -> &'static str {
        "paid_storage_auto"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.paid_storage_auto")
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

struct TaskInfo {
    cabinet: String,
    task_id: String,
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Старт автоотчёта");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 7)?;
    let intervals = split_intervals(start, end, 8);
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let wb_tokens = common::load_wb_tokens()?;
    let create_p = rate::profile_create();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let intervals = intervals.clone();
            let create_p = create_p.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<String>::new()));
                }
                let mut task_ids = Vec::new();
                for (from, to) in &intervals {
                    match api::create_task(
                        &ctx.request,
                        &cabinet,
                        token.trim(),
                        rate::SCOPE_CREATE,
                        &create_p,
                        *from,
                        *to,
                    )
                    .await
                    {
                        Ok(task_id) => {
                            ctx.log.info(format!(
                                "Кабинет «{cabinet}»: создан taskId={task_id}"
                            ));
                            task_ids.push(task_id);
                        }
                        Err(e) => ctx.log.warn(e),
                    }
                }
                Ok((cabinet, task_ids))
            }
        },
    )
    .await?;

    let mut tasks: Vec<TaskInfo> = Vec::new();
    for (cabinet, task_ids) in parts {
        for task_id in task_ids {
            tasks.push(TaskInfo { cabinet: cabinet.clone(), task_id });
        }
    }

    if tasks.is_empty() {
        return Ok(common::empty_data_output());
    }

    wait_done(ctx, &wb_tokens, &tasks).await?;

    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut all_rows = vec![common::selected_titles(&fields)];
    let dl_p = rate::profile_download();
    let mut total = 0usize;

    for t in &tasks {
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &t.cabinet);
        if token.trim().is_empty() {
            continue;
        }
        match api::download_report(
            &ctx.request,
            &t.cabinet,
            token.trim(),
            rate::SCOPE_DOWNLOAD,
            &dl_p,
            &t.task_id,
        )
        .await
        {
            Ok(items) => {
                for it in &items {
                    let row: Vec<String> = selected
                        .iter()
                        .map(|f| report_cell(f.name.as_str(), &t.cabinet, &t.task_id, it))
                        .collect();
                    all_rows.push(row);
                    total += 1;
                }
            }
            Err(e) => ctx.log.warn(e),
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Автоотчёт: записано {total} строк"),
    })
}

async fn wait_done(
    ctx: &ScriptContext,
    wb_tokens: &common::WbTokenMap,
    tasks: &[TaskInfo],
) -> Result<(), String> {
    let status_p = rate::profile_status();
    sleep(Duration::from_secs(10)).await;
    loop {
        let mut pending = Vec::new();
        for t in tasks {
            let token = common::resolve_wb_token_with_map(ctx, wb_tokens, &t.cabinet);
            if token.trim().is_empty() {
                pending.push(format!("{}:{}", t.cabinet, t.task_id));
                continue;
            }
            let st = api::task_status(
                &ctx.request,
                &t.cabinet,
                token.trim(),
                rate::SCOPE_STATUS,
                &status_p,
                &t.task_id,
            )
            .await
            .unwrap_or_else(|e| format!("ERROR:{e}"));
            if is_failed_status(&st) {
                return Err(format!(
                    "Задание {}/{} завершилось с ошибкой: {st}",
                    t.cabinet, t.task_id
                ));
            }
            if !is_done_status(&st) {
                pending.push(format!("{}:{} ({st})", t.cabinet, t.task_id));
            }
        }
        if pending.is_empty() {
            ctx.log.info("Все задания платного хранения готовы");
            return Ok(());
        }
        ctx.log.info(format!(
            "Ожидание готовности: {} заданий…",
            pending.len()
        ));
        sleep(Duration::from_secs(10)).await;
    }
}
