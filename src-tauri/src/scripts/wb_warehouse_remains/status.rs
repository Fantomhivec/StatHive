//! `warehouse_remains_status` — статусы в output range.

use super::api;
use crate::core::rate_profiles::warehouse_remains_status as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name};
use std::future::Future;
use std::pin::Pin;

pub struct WarehouseRemainsStatusScript;

impl Script for WarehouseRemainsStatusScript {
    fn key(&self) -> &'static str {
        "warehouse_remains_status"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouse_remains_status")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
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
    ctx.log.info("Старт");
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
    // Unify ranges with paid_storage / nm_report UI fields.
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    let task_r = param(ctx, "paid_storage_status_task_id_range").unwrap_or("");
    let out_r = param(ctx, "paid_storage_status_output_range").unwrap_or("");
    if cab_r.is_empty() || task_r.is_empty() || out_r.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов, taskId и вывода статусов".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(&client, src_id, src_sheet, &[cab_r, task_r]).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let tasks = cols.get(1).cloned().unwrap_or_default();
    if cabs.len() != tasks.len() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: format!(
                "Длины диапазонов кабинетов ({}) и taskId ({}) должны совпадать",
                cabs.len(),
                tasks.len()
            ),
        });
    }

    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut statuses: Vec<Vec<String>> = Vec::with_capacity(tasks.len());

    for i in 0..tasks.len() {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let cab_raw = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let task_id = tasks.get(i).map(|s| s.trim()).unwrap_or("");
        if task_id.is_empty() {
            statuses.push(vec![String::new()]);
            continue;
        }
        if cab_raw.is_empty() {
            statuses.push(vec!["ERROR: Кабинет не указан".into()]);
            continue;
        }
        let label = normalize_cabinet_name(cab_raw);
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &label);
        if token.trim().is_empty() {
            statuses.push(vec!["ERROR: нет токена".into()]);
            continue;
        }
        match api::task_status(
            &ctx.request,
            &label,
            token.trim(),
            rate::SCOPE,
            &profile,
            task_id,
        )
        .await
        {
            Ok(s) => statuses.push(vec![s]),
            Err(e) => statuses.push(vec![format!("ERROR: {e}")]),
        }
    }

    let a1 = sheet_ranges::a1_with_sheet(&dest_sheet, out_r);
    client
        .write_values(&dest_id, &a1, statuses.clone())
        .await
        .map_err(|e| format!("Запись статусов: {e}"))?;

    Ok(ScriptOutput {
        rows_written: statuses.len(),
        message: format!("Записано статусов: {}", statuses.len()),
    })
}
