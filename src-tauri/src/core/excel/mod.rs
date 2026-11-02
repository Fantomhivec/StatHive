//! Excel write helpers (scripts / calc export).

use rust_xlsxwriter::{Format, Workbook, XlsxError};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleSheetWrite {
    pub path: String,
    #[serde(default = "default_sheet")]
    pub sheet_name: String,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

fn default_sheet() -> String {
    "Sheet1".into()
}

fn map_xlsx(e: XlsxError) -> String {
    format!("Excel write: {e}")
}

/// Записать простую книгу (один лист).
pub fn write_simple_xlsx(args: &SimpleSheetWrite) -> Result<String, String> {
    let path = Path::new(args.path.trim());
    if args.path.trim().is_empty() {
        return Err("Путь к файлу пуст".into());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let mut workbook = Workbook::new();
    let sheet_name = if args.sheet_name.trim().is_empty() {
        "Sheet1"
    } else {
        args.sheet_name.trim()
    };
    let sheet = workbook.add_worksheet();
    sheet.set_name(sheet_name).map_err(map_xlsx)?;

    let header_fmt = Format::new().set_bold();
    let mut row_idx: u32 = 0;
    if !args.headers.is_empty() {
        for (col, h) in args.headers.iter().enumerate() {
            sheet
                .write_string_with_format(row_idx, col as u16, h, &header_fmt)
                .map_err(map_xlsx)?;
        }
        row_idx += 1;
    }
    for row in &args.rows {
        for (col, cell) in row.iter().enumerate() {
            sheet
                .write_string(row_idx, col as u16, cell)
                .map_err(map_xlsx)?;
        }
        row_idx += 1;
    }

    workbook.save(path).map_err(map_xlsx)?;
    Ok(path.display().to_string())
}
