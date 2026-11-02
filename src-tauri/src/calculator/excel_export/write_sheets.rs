use super::data::DataTable;
use super::formulas::SHEET_MAP;
use super::write_cell::write_data_cell;
use rust_xlsxwriter::{Format, Formula, Workbook, Worksheet};

pub fn write_data_sheet(wb: &mut Workbook, name: &str, table: &DataTable) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name(name).map_err(|e| e.to_string())?;
    let header_fmt = Format::new().set_bold();
    for (col, h) in table.headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, h, &header_fmt)
            .map_err(|e| e.to_string())?;
    }
    for (r, row) in table.rows.iter().enumerate() {
        for (col, h) in table.headers.iter().enumerate() {
            if let Some(val) = row.get(h) {
                if !val.is_empty() {
                    write_data_cell(sheet, (r + 1) as u32, col as u16, h, val)?;
                }
            }
        }
    }
    if !table.headers.is_empty() {
        let _ = sheet.set_freeze_panes(1, 1);
        // AutoFitColumns(8, 40) — базовые ширины без макросов.
        for col in 0..table.headers.len().min(40) {
            let w = (table.headers[col].chars().count() as f64 + 2.0).clamp(8.0, 40.0);
            sheet.set_column_width(col as u16, w).ok();
        }
    }
    Ok(())
}

pub fn write_map_sheet(
    wb: &mut Workbook,
    wb_table: &DataTable,
    oz_table: &DataTable,
) -> Result<(), String> {
    let sheet = wb.add_worksheet();
    sheet.set_name(SHEET_MAP).map_err(|e| e.to_string())?;
    let header_fmt = Format::new().set_bold();
    sheet
        .write_string_with_format(0, 0, "Артикул", &header_fmt)
        .map_err(|e| e.to_string())?;
    sheet
        .write_string_with_format(0, 1, "Площадка", &header_fmt)
        .map_err(|e| e.to_string())?;
    let text = Format::new().set_num_format("@");
    let mut r: u32 = 1;
    for row in &wb_table.rows {
        let art = row.get("Артикул").map(|s| s.as_str()).unwrap_or("");
        if art.trim().is_empty() {
            continue;
        }
        sheet
            .write_string_with_format(r, 0, art, &text)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(r, 1, "WB")
            .map_err(|e| e.to_string())?;
        r += 1;
    }
    for row in &oz_table.rows {
        let art = row
            .get("SKU")
            .or_else(|| row.get("Артикул"))
            .map(|s| s.as_str())
            .unwrap_or("");
        if art.trim().is_empty() {
            continue;
        }
        sheet
            .write_string_with_format(r, 0, art, &text)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(r, 1, "OZON")
            .map_err(|e| e.to_string())?;
        r += 1;
    }
    let _ = sheet.set_freeze_panes(1, 0);
    // AutoFitColumns(8, 24) — фиксированные разумные ширины.
    sheet.set_column_width(0, 16.0).ok();
    sheet.set_column_width(1, 10.0).ok();
    Ok(())
}

pub fn write_string(ws: &mut Worksheet, row: u32, col: u16, text: &str, fmt: &Format) -> Result<(), String> {
    ws.write_string_with_format(row, col, text, fmt)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_formula(ws: &mut Worksheet, row: u32, col: u16, formula: &str, fmt: &Format) -> Result<(), String> {
    ws.write_formula_with_format(row, col, Formula::new(formula), fmt)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn merge_write(
    ws: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
    text: &str,
    fmt: &Format,
) -> Result<(), String> {
    ws.merge_range(first_row, first_col, last_row, last_col, text, fmt)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn merge_formula(
    ws: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
    formula: &str,
    fmt: &Format,
) -> Result<(), String> {
    // merge_range with empty then write formula into top-left
    ws.merge_range(first_row, first_col, last_row, last_col, "", fmt)
        .map_err(|e| e.to_string())?;
    ws.write_formula_with_format(first_row, first_col, Formula::new(formula), fmt)
        .map_err(|e| e.to_string())?;
    Ok(())
}

