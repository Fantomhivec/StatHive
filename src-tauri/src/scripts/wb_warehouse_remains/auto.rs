//! `warehouse_remains_auto` — create → poll → download.

use super::api::{
    self, download_fields, flatten_report_items, is_done_status, is_failed_status, report_cell,
    GroupBy,
};
use crate::core::rate_profiles::warehouse_remains_auto as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;
use tokio::time::{sleep, Duration};

pub struct WarehouseRemainsAutoScript;

impl Script for WarehouseRemainsAutoScript {
    fn key(&self) -> &'static str {
        "warehouse_remains_auto"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouse_remains_auto")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        download_fields()
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

fn param_bool(ctx: &ScriptContext, key: &str, default: bool) -> bool {
    match ctx.params.get(key).map(|s| s.trim().to_ascii_lowercase()) {
        Some(s) if s == "1" || s == "true" || s == "yes" || s == "да" => true,
        Some(s) if s == "0" || s == "false" || s == "no" || s == "нет" => false,
        _ => default,
    }
}

fn group_by_from_ctx(ctx: &ScriptContext) -> GroupBy {
    GroupBy {
        nm: param_bool(ctx, "group_by_nm", true),
        size: param_bool(ctx, "group_by_size", true),
        brand: param_bool(ctx, "group_by_brand", false),
        subject: param_bool(ctx, "group_by_subject", false),
        sa: param_bool(ctx, "group_by_sa", false),
        barcode: param_bool(ctx, "group_by_barcode", false),
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Старт автоотчёта warehouse_remains");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let g = group_by_from_ctx(ctx);
    let wb_tokens = common::load_wb_tokens()?;
    let create_p = rate::profile_create();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let create_p = create_p.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, None::<String>));
                }
                match api::create_task(
                    &ctx.request,
                    &cabinet,
                    token.trim(),
                    rate::SCOPE_CREATE,
                    &create_p,
                    g,
                )
                .await
                {
                    Ok(task_id) => {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: создан taskId={task_id}"
                        ));
                        Ok((cabinet, Some(task_id)))
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

    let mut tasks: Vec<TaskInfo> = Vec::new();
    for (cabinet, task_id) in parts {
        if let Some(task_id) = task_id {
            tasks.push(TaskInfo { cabinet, task_id });
        }
    }
    if tasks.is_empty() {
        return Ok(common::empty_data_output());
    }

    wait_done(ctx, &wb_tokens, &tasks).await?;

    let fields = common::resolve_fields(ctx, download_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut all_rows = vec![common::selected_titles(&fields)];
    let dl_p = rate::profile_download();
    let mut total = 0usize;

    for t in &tasks {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
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
                for it in flatten_report_items(&items) {
                    let row: Vec<String> = selected
                        .iter()
                        .map(|f| report_cell(f.name.as_str(), &t.cabinet, &t.task_id, &it))
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
        message: format!("Автоотчёт warehouse_remains: {total} строк"),
    })
}

async fn wait_done(
    ctx: &ScriptContext,
    wb_tokens: &common::WbTokenMap,
    tasks: &[TaskInfo],
) -> Result<(), String> {
    let status_p = rate::profile_status();
    sleep(Duration::from_secs(10)).await;
    // Cap poll loops (~15 min at 10s) — avoid infinite wait on stuck tasks.
    for _ in 0..90 {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
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
            ctx.log.info("Все задания warehouse_remains готовы");
            return Ok(());
        }
        ctx.log.info(format!(
            "Ожидание готовности: {} заданий…",
            pending.len()
        ));
        sleep(Duration::from_secs(10)).await;
    }
    Err("Таймаут ожидания готовности заданий warehouse_remains".into())
}
