//! `warehouse_remains_download`.

use super::api::{self, download_fields, flatten_report_items, report_cell};
use crate::core::rate_profiles::warehouse_remains_download as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct WarehouseRemainsDownloadScript;

impl Script for WarehouseRemainsDownloadScript {
    fn key(&self) -> &'static str {
        "warehouse_remains_download"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.warehouse_remains_download")
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
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    let task_r = param(ctx, "paid_storage_download_task_id_range").unwrap_or("");
    if cab_r.is_empty() || task_r.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов и taskId".into(),
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

    let mut by_cab: HashMap<String, Vec<String>> = HashMap::new();
    let mut labels: HashMap<String, String> = HashMap::new();
    for i in 0..tasks.len() {
        let cab = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let tid = tasks.get(i).map(|s| s.trim()).unwrap_or("");
        if cab.is_empty() || tid.is_empty() {
            continue;
        }
        let key = normalize_cabinet_name(cab).to_ascii_lowercase();
        labels.entry(key.clone()).or_insert_with(|| cab.to_string());
        by_cab.entry(key).or_default().push(tid.to_string());
    }

    let fields = common::resolve_fields(ctx, download_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut all_rows = vec![common::selected_titles(&fields)];
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut total = 0usize;

    for (norm, task_ids) in &by_cab {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let label = labels.get(norm).cloned().unwrap_or_else(|| norm.clone());
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &label);
        if token.trim().is_empty() {
            ctx.log.warn(format!("Кабинет «{label}»: нет токена"));
            continue;
        }
        for tid in task_ids {
            ctx.check_cancelled().map_err(|e| e.to_string())?;
            match api::download_report(
                &ctx.request,
                &label,
                token.trim(),
                rate::SCOPE,
                &profile,
                tid,
            )
            .await
            {
                Ok(items) => {
                    for it in flatten_report_items(&items) {
                        let row: Vec<String> = selected
                            .iter()
                            .map(|f| report_cell(f.name.as_str(), &label, tid, &it))
                            .collect();
                        all_rows.push(row);
                        total += 1;
                    }
                }
                Err(e) => ctx.log.warn(e),
            }
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк отчёта на лист «{dest_sheet}»"),
    })
}
