//! Excel: «Калькулятор» + «Соответствие» + «Данные ВБ» + «Данные OZON» (паритет).

mod calc_cf;
mod calc_layout;
mod calc_sheet;
mod calc_sheet_left;
mod calc_sheet_right;
mod data;
mod formulas;
mod overlay;
mod styles;
mod write_cell;
mod write_sheets;

use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::sheet_util::SheetCache;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::log::AppLog;
use formulas::{SHEET_CALC, SHEET_OZ, SHEET_WB};
use rust_xlsxwriter::Workbook;
use serde_json::{json, Value};
use std::path::Path;

pub async fn export_excel_logged(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    file_path: &str,
    log: Option<&AppLog>,
) -> Result<Value, String> {
    let path = Path::new(file_path.trim());
    if file_path.trim().is_empty() {
        return Err("Не указан путь к файлу.".into());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let mut warnings: Vec<String> = Vec::new();
    let mut cache_wb = SheetCache::new();
    let mut cache_oz = SheetCache::new();
    let mut warnings_wb = Vec::new();
    let mut warnings_oz = Vec::new();

    if let Some(l) = log {
        run_log::calc_info(l, "[Калькулятор] Excel: чтение сводов…");
    }

    // Параллельно: разные SheetCache, один gs (immutable).
    let (wb_table, oz_table) = tokio::join!(
        data::build_wb_table(profile, gs, &mut cache_wb, &mut warnings_wb),
        data::build_oz_table(profile, gs, &mut cache_oz, &mut warnings_oz),
    );
    warnings.append(&mut warnings_wb);
    warnings.append(&mut warnings_oz);
    if wb_table.rows.is_empty() && oz_table.rows.is_empty() {
        return Ok(json!({
            "ok": false,
            "cancelled": false,
            "path": Value::Null,
            "warnings": if warnings.is_empty() {
                vec![
                    "Нет сводов «Карточка WB» / «Карточка Ozon». Сначала создайте или обновите таблицы."
                        .to_string(),
                ]
            } else {
                warnings
            }
        }));
    }

    if let Some(l) = log {
        run_log::calc_info(l, "[Калькулятор] Excel: запись файла…");
    }

    let mut workbook = Workbook::new();
    // Порядок Калькулятор → Соответствие → Данные ВБ → Данные OZON
    calc_sheet::write_calculator_sheet(
        &mut workbook,
        profile,
        wb_table.rows.len(),
        oz_table.rows.len(),
    )?;
    write_sheets::write_map_sheet(&mut workbook, &wb_table, &oz_table)?;
    write_sheets::write_data_sheet(&mut workbook, SHEET_WB, &wb_table)?;
    write_sheets::write_data_sheet(&mut workbook, SHEET_OZ, &oz_table)?;

    workbook
        .save(path)
        .map_err(|e| format!("Excel write: {e}"))?;

    if let Some(l) = log {
        run_log::calc_info(l, format!("[Калькулятор] Excel сохранён: {}", path.display()));
    }

    open_export_folder(path);

    Ok(json!({
        "ok": true,
        "cancelled": false,
        "path": path.display().to_string(),
        "warnings": warnings,
        "wbRows": wb_table.rows.len(),
        "ozRows": oz_table.rows.len(),
        "sheetCalc": SHEET_CALC,
        "sheetOz": SHEET_OZ,
    }))
}

/// Открыть папку выгрузки и выделить файл (как ожидает UX после Excel).
fn open_export_folder(path: &Path) {
    if tauri_plugin_opener::reveal_item_in_dir(path).is_ok() {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = tauri_plugin_opener::open_path(dir, None::<&str>);
    }
}
