//! Запись ячеек data-листов (паритет `WriteCell`).
//! Числа — как `number`, иначе `N()` в формулах прибыли даёт 0 на тексте «20935».

use crate::calculator::sheet_util::try_parse_number;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rust_xlsxwriter::{ExcelDateTime, Format, Worksheet};
use std::sync::OnceLock;

const FMT_DATE: &str = "dd.MM.yyyy";

struct CellFormats {
    text: Format,
    date: Format,
    general: Format,
}

fn cell_formats() -> &'static CellFormats {
    static F: OnceLock<CellFormats> = OnceLock::new();
    F.get_or_init(|| CellFormats {
        text: Format::new().set_num_format("@"),
        date: Format::new().set_num_format(FMT_DATE),
        general: Format::new().set_num_format("General"),
    })
}

pub fn write_data_cell(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    header: &str,
    val: &str,
) -> Result<(), String> {
    let s = val.trim();
    if s.is_empty() {
        return Ok(());
    }
    let fmts = cell_formats();
    if is_article_header(header) {
        ws.write_string_with_format(row, col, s, &fmts.text)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if should_try_date(header, s) {
        if let Some(d) = parse_sheet_date(s) {
            let excel_dt = ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8)
                .map_err(|e| e.to_string())?;
            ws.write_datetime_with_format(row, col, &excel_dt, &fmts.date)
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    if let Some(n) = try_parse_number(s) {
        if !looks_like_id(s) && allow_numeric_header(header) {
            // WriteCell: Numberformat = General (не «0.##» — иначе «173,» в ru-RU).
            ws.write_number_with_format(row, col, n, &fmts.general)
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    ws.write_string(row, col, s).map_err(|e| e.to_string())?;
    Ok(())
}

fn should_try_date(header: &str, s: &str) -> bool {
    !looks_like_plain_number(s) && (is_date_header(header) || looks_like_date_text(s))
}

fn is_date_header(header: &str) -> bool {
    let h = header.to_lowercase();
    h.contains("дата") || h.contains("date")
}

fn looks_like_plain_number(s: &str) -> bool {
    let t = s.trim().trim_end_matches(['%', ' ', '\u{00a0}']);
    if looks_like_date_text(t) {
        return false;
    }
    let mut has_digit = false;
    for c in t.chars() {
        if c.is_ascii_digit() {
            has_digit = true;
        } else if !matches!(c, ',' | '.' | '-' | '+') {
            return false;
        }
    }
    has_digit
}

fn looks_like_date_text(s: &str) -> bool {
    let t = s.trim();
    let parts: Vec<_> = t.split(['.', '/', '-']).collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit() || c == ' '))
}

fn parse_sheet_date(s: &str) -> Option<NaiveDate> {
    let t = s.trim();
    const DATE_FMTS: &[&str] = &["%d.%m.%Y", "%Y-%m-%d", "%d/%m/%Y", "%d.%m.%y"];
    for f in DATE_FMTS {
        if let Ok(d) = NaiveDate::parse_from_str(t, f) {
            return Some(d);
        }
    }
    const DT_FMTS: &[&str] = &[
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ];
    for f in DT_FMTS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(t, f) {
            return Some(dt.date());
        }
    }
    if t.len() >= 10 {
        if let Ok(d) = NaiveDate::parse_from_str(&t[..10], "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
}

fn is_article_header(header: &str) -> bool {
    let h = header.to_lowercase();
    h == "артикул" || h == "sku" || h.contains("артикул")
}

fn looks_like_id(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 12 && t.bytes().all(|b| b.is_ascii_digit())
}

fn allow_numeric_header(header: &str) -> bool {
    let h = header.to_lowercase();
    !h.contains("фото")
        && !h.contains("статус")
        && !h.contains("кабинет")
        && !h.contains("бренд")
        && !h.contains("наименован")
        && !h.contains("категор")
        && !h.contains("артикул")
        && h != "sku"
        && !h.contains("offer")
        && !h.contains("тн-вэд")
        && !h.contains("номер")
        && !h.contains("есть реклама")
        && !h.contains("дата")
        && !h.contains("date")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_header_allows_number() {
        assert!(allow_numeric_header("Цена после скидки"));
        assert!(allow_numeric_header("Закуп"));
        assert!(allow_numeric_header("Логистика"));
        assert!(allow_numeric_header("Комиссия FBS"));
    }

    #[test]
    fn article_stays_text() {
        assert!(is_article_header("Артикул"));
        assert!(!allow_numeric_header("Артикул ПСТ"));
        assert!(looks_like_id("123456789012"));
        assert!(!looks_like_id("158883495"));
    }

    #[test]
    fn parses_ru_date() {
        assert_eq!(
            parse_sheet_date("19.10.2026").map(|d| d.to_string()),
            Some("2026-10-19".into())
        );
    }
}
