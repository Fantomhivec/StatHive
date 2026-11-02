//! Разбор колонок Массива (заголовки / nm / chrt).

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::script::ScriptContext;

pub(crate) async fn try_detect_ranges_by_header(
    ctx: &ScriptContext,
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
) -> Result<Option<(String, String, String)>, String> {
    let sheet_q = sheet_name.replace('\'', "''");
    ctx.log.info("Авто-поиск колонок по заголовкам (строка 1)…");
    let rows = client
        .read_values(spreadsheet_id, &format!("'{sheet_q}'!1:1"))
        .await
        .map_err(|e| format!("Чтение заголовков: {e}"))?;
    let Some(header) = rows.first() else {
        return Ok(None);
    };
    let cab_col = find_col(header, &["Кабинет", "cabinet"]);
    let nm_col = find_col(header, &["Артикул WB", "Артикул ВБ", "nmID", "nmId", "NmID"]);
    let chrt_col = find_col(
        header,
        &[
            "Размеры: chrtID",
            "chrtID (через ;)",
            "chrtID",
            "chrtId",
            "ID размера",
        ],
    );
    let (Some(c), Some(n), Some(h)) = (cab_col, nm_col, chrt_col) else {
        ctx.log.warn(format!(
            "Авто-поиск колонок: cabinet={cab_col:?}, nmId={nm_col:?}, chrtId={chrt_col:?} (нужны все три)"
        ));
        return Ok(None);
    };
    let ranges = (
        format!("{}2:{}", col_to_letter(c + 1), col_to_letter(c + 1)),
        format!("{}2:{}", col_to_letter(n + 1), col_to_letter(n + 1)),
        format!("{}2:{}", col_to_letter(h + 1), col_to_letter(h + 1)),
    );
    ctx.log.info(format!(
        "Колонки найдены по заголовкам: cabinets={}, nmId={}, chrtId={}",
        ranges.0, ranges.1, ranges.2
    ));
    Ok(Some(ranges))
}

pub(crate) fn flatten_col(rows: &[Vec<String>]) -> Vec<String> {
    rows.iter()
        .map(|row| row.first().cloned().unwrap_or_default())
        .collect()
}

pub(crate) fn col_to_letter(col_1based: usize) -> String {
    let mut n = col_1based;
    let mut s = String::new();
    while n > 0 {
        n -= 1;
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    s
}

/// Сначала точное совпадение, затем contains — FindHeaderColumn.
pub(crate) fn find_col(header: &[String], aliases: &[&str]) -> Option<usize> {
    for (i, h) in header.iter().enumerate() {
        let t = h.trim();
        if t.is_empty() {
            continue;
        }
        if aliases.iter().any(|a| t.eq_ignore_ascii_case(a)) {
            return Some(i);
        }
    }
    for (i, h) in header.iter().enumerate() {
        let t = h.trim();
        if t.is_empty() {
            continue;
        }
        if aliases
            .iter()
            .any(|a| t.to_ascii_lowercase().contains(&a.to_ascii_lowercase()))
        {
            return Some(i);
        }
    }
    None
}

/// `NormalizeIntegerText` (цифры / float из Sheets).
pub(crate) fn parse_i64(s: &str) -> Option<i64> {
    let t = s.trim().trim_start_matches('\'');
    if t.is_empty() {
        return None;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        let trimmed = t.trim_start_matches('0');
        let norm = if trimmed.is_empty() { "0" } else { trimmed };
        return norm.parse().ok();
    }
    if let Ok(n) = t.parse::<i64>() {
        return Some(n);
    }
    t.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|d| d.is_finite())
        .map(|d| d.round() as i64)
}

pub(crate) fn parse_chrt_list(raw: &str) -> Vec<i64> {
    raw.split(|c| c == ';' || c == ',' || c == '|' || c == ' ')
        .filter_map(|p| parse_i64(p).filter(|n| *n > 0))
        .collect()
}


pub(crate) fn param(ctx: &ScriptContext, key: &str) -> Option<String> {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
