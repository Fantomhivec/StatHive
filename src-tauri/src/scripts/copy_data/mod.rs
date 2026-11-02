//! `copy_data` — копирование диапазона Google Sheets (source → dest sheet).

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges;
use std::future::Future;
use std::pin::Pin;

pub struct CopyDataScript;

impl Script for CopyDataScript {
    fn key(&self) -> &'static str {
        "copy_data"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.copy_data")
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
    ctx.log.info("Скрипт «Копирование данных»: старт");
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
    let src_range = param(ctx, "cabinets_source_range").unwrap_or("").trim();
    if src_range.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите исходный диапазон (поле диапазона кабинетов = A1-диапазон)".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let a1_src = sheet_ranges::a1_with_sheet(src_sheet, src_range);
    let values = client
        .read_values(src_id, &a1_src)
        .await
        .map_err(|e| format!("Чтение {a1_src}: {e}"))?;
    if values.is_empty() {
        return Ok(common::empty_data_output());
    }
    let rows = values.len();
    common::write_sheet_rows_with(ctx, &client, &dest_id, &dest_sheet, values).await?;
    Ok(ScriptOutput {
        rows_written: rows,
        message: format!("Скопировано строк: {rows} → «{dest_sheet}»"),
    })
}
