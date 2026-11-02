//! `warehouse_remains_create_task`.

use super::api::{self, create_fields, GroupBy};
use crate::core::rate_profiles::warehouse_remains_create_task as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;

pub struct WarehouseRemainsCreateTaskScript;

impl Script for WarehouseRemainsCreateTaskScript {
    fn key(&self) -> &'static str {
        "warehouse_remains_create_task"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouse_remains_create_task")
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
    ctx.log.info("Warehouse remains create: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let g = group_by_from_ctx(ctx);
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
                match api::create_task(
                    &ctx.request,
                    &cabinet,
                    token.trim(),
                    rate::SCOPE,
                    &profile,
                    g,
                )
                .await
                {
                    Ok(task_id) => {
                        ctx.log
                            .info(format!("Кабинет «{cabinet}»: taskId={task_id}"));
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

    let created_at = sheet_dates::format_datetime(chrono::Local::now().naive_local());
    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, task_id) in parts {
        let Some(task_id) = task_id else { continue };
        total += 1;
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(match f.name.as_str() {
                "cabinet" => cabinet.clone(),
                "taskId" => task_id.clone(),
                "createdAt" => created_at.clone(),
                _ => String::new(),
            });
        }
        rows.push(row);
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Создано {total} заданий warehouse_remains на «{sheet_name}»"),
    })
}
