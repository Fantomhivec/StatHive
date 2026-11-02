//! `cells_count` — подсчёт ячеек Google Sheets (лог only, без записи).

use crate::core::google::http as google_http;
use crate::core::google::sheets::{quote_sheet, to_a1_column, truncate};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const MAX_CELLS: f64 = 10_000_000.0;

pub struct CellsCountScript;

impl Script for CellsCountScript {
    fn key(&self) -> &'static str {
        "cells_count"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cells_count")
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

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Подсчет ячеек»: старт");
    let (spreadsheet_id, _) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let count_filled = ctx
        .params
        .get("cells_count_count_filled")
        .map(|s| s.eq_ignore_ascii_case("true") || s == "1")
        .unwrap_or(false);

    let client = common::google_sheets_client(ctx).await?;
    let meta = sheet_grid_meta(&client, &spreadsheet_id).await?;
    if meta.is_empty() {
        return Err("Таблица не найдена или без листов".into());
    }

    ctx.log.info(format!("ID таблицы: {spreadsheet_id}"));
    ctx.log.info(format!("Листов: {}", meta.len()));
    let names: Vec<&str> = meta.iter().map(|(n, _, _)| n.as_str()).collect();
    ctx.log.info(format!("Наименования: {}", names.join(", ")));

    let mut total_cells: i64 = 0;
    let mut filled_cells: i64 = 0;
    for (i, (name, rows, cols)) in meta.iter().enumerate() {
        ctx.check_cancelled()?;
        let sheet_total = (*rows as i64) * (*cols as i64);
        total_cells += sheet_total;
        ctx.log.info(format!(
            "Лист {} из {}: «{name}» — {rows}×{cols} = {sheet_total} ячеек",
            i + 1,
            meta.len()
        ));
        if count_filled && *rows > 0 && *cols > 0 {
            let filled =
                count_filled_on_sheet(&client, &spreadsheet_id, name, *rows, *cols).await?;
            filled_cells += filled as i64;
            ctx.log.info(format!(
                "Лист {} из {}: «{name}» — заполнено {filled} ячеек",
                i + 1,
                meta.len()
            ));
        }
    }

    let total_pct = if total_cells > 0 {
        total_cells as f64 / MAX_CELLS * 100.0
    } else {
        0.0
    };
    ctx.log.info(format!("Общее количество ячеек: {total_cells}"));
    if count_filled {
        let filled_pct = filled_cells as f64 / MAX_CELLS * 100.0;
        ctx.log.info(format!("Заполнено ячеек: {filled_cells}"));
        ctx.log.info(format!(
            "% по общим ячейкам: {total_pct:.2}% (от {MAX_CELLS})"
        ));
        ctx.log.info(format!(
            "% с учётом используемых: {filled_pct:.2}% (от {MAX_CELLS})"
        ));
    } else {
        ctx.log.info(format!(
            "% использования от максимума: {total_pct:.2}% (от {MAX_CELLS})"
        ));
    }

    Ok(ScriptOutput {
        rows_written: 0,
        message: format!("Ячеек в документе: {total_cells}"),
    })
}

async fn sheet_grid_meta(
    client: &crate::core::google::sheets::GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<Vec<(String, usize, usize)>, String> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(title,gridProperties(rowCount,columnCount))"
    );
    let token = client.access_token.clone();
    let http = client.http.clone();
    let (status, text) = google_http::exchange_with_backoff(|| {
        http.get(&url).bearer_auth(&token).send()
    })
    .await?;
    if !status.is_success() {
        return Err(format!(
            "Sheets meta HTTP {status}: {}",
            truncate(&text, 200)
        ));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
        for sh in sheets {
            let title = sh
                .pointer("/properties/title")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let rows = sh
                .pointer("/properties/gridProperties/rowCount")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as usize;
            let cols = sh
                .pointer("/properties/gridProperties/columnCount")
                .and_then(|x| x.as_u64())
                .unwrap_or(0) as usize;
            if !title.is_empty() {
                out.push((title, rows, cols));
            }
        }
    }
    Ok(out)
}

async fn count_filled_on_sheet(
    client: &crate::core::google::sheets::GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet: &str,
    row_count: usize,
    col_count: usize,
) -> Result<usize, String> {
    let last_col = to_a1_column(col_count);
    let a1 = format!("{}!A1:{last_col}{row_count}", quote_sheet(sheet));
    let values = client.read_values(spreadsheet_id, &a1).await?;
    let mut n = 0usize;
    for row in values {
        for cell in row {
            if !cell.trim().is_empty() {
                n += 1;
            }
        }
    }
    Ok(n)
}
