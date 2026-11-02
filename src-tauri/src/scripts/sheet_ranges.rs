//! Чтение выровненных колонок из Google Sheets (мутации / ТН ВЭД / реклама).

use crate::core::google_sheets::GoogleSheetsClient;
use std::collections::{HashMap, HashSet};

/// A1 с именем листа: `'Лист'!A2:A`.
pub fn a1_with_sheet(sheet_name: &str, range: &str) -> String {
    let sheet_q = sheet_name.replace('\'', "''");
    format!("'{sheet_q}'!{}", range.trim())
}

/// Первая ячейка каждой строки → колонка строк.
pub fn flatten_first_col(values: &[Vec<String>]) -> Vec<String> {
    values
        .iter()
        .map(|row| row.first().map(|s| s.trim().to_string()).unwrap_or_default())
        .collect()
}

/// Несколько диапазонов одним batchGet; каждый → flatten первой колонки.
pub async fn read_first_cols(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    ranges: &[&str],
) -> Result<Vec<Vec<String>>, String> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    let a1: Vec<String> = ranges
        .iter()
        .map(|r| a1_with_sheet(sheet_name, r))
        .collect();
    let refs: Vec<&str> = a1.iter().map(|s| s.as_str()).collect();
    let batches = client
        .read_values_batch(spreadsheet_id.trim(), &refs)
        .await
        .map_err(|e| format!("Чтение диапазонов: {e}"))?;
    Ok(batches
        .into_iter()
        .map(|m| flatten_first_col(&m))
        .collect())
}

/// Полные строки диапазона (для media links: несколько колонок).
pub async fn read_matrix(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    range: &str,
) -> Result<Vec<Vec<String>>, String> {
    let a1 = a1_with_sheet(sheet_name, range);
    client
        .read_values(spreadsheet_id.trim(), &a1)
        .await
        .map_err(|e| format!("Чтение диапазона: {e}"))
}

pub fn normalize_cabinet_name(raw: &str) -> String {
    let mut t = raw.trim().to_string();
    if t.to_ascii_uppercase().starts_with("API_KEY_") {
        t = t[8..].to_string();
    }
    t.trim().to_string()
}

pub fn parse_i64_cell(raw: &str) -> Option<i64> {
    let s = raw.trim().replace('\u{00a0}', "").replace(' ', "").replace(',', ".");
    if s.is_empty() {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        return Some(n);
    }
    s.parse::<f64>().ok().map(|f| f as i64)
}

/// Пары (кабинет, id кампании как строка) из двух колонок.
pub async fn load_cabinet_id_pairs(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    cab_range: &str,
    id_range: &str,
) -> Result<Vec<(String, String)>, String> {
    let cols = read_first_cols(client, spreadsheet_id, sheet_name, &[cab_range, id_range]).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let ids = cols.get(1).cloned().unwrap_or_default();
    if ids.is_empty() {
        return Err("В диапазоне ID кампаний нет данных".into());
    }
    if ids.len() != cabs.len() {
        return Err(format!(
            "Количество ID кампаний ({}) не соответствует количеству кабинетов ({})",
            ids.len(),
            cabs.len()
        ));
    }
    Ok(cabs
        .into_iter()
        .zip(ids)
        .filter(|(c, i)| !c.trim().is_empty() && !i.trim().is_empty())
        .map(|(c, i)| (normalize_cabinet_name(&c), i.trim().to_string()))
        .collect())
}

/// Тройки (кабинет, advert_id, optional nm) — nm-колонка опциональна.
pub async fn load_cabinet_advert_nm_rows(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    cab_range: &str,
    id_range: &str,
    nm_range: &str,
) -> Result<Vec<(String, i64, Option<i64>)>, String> {
    let ranges: Vec<&str> = if nm_range.trim().is_empty() {
        vec![cab_range, id_range]
    } else {
        vec![cab_range, id_range, nm_range]
    };
    let cols = read_first_cols(client, spreadsheet_id, sheet_name, &ranges).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let ids = cols.get(1).cloned().unwrap_or_default();
    let nms = cols.get(2).cloned().unwrap_or_default();
    let n = cabs.len().max(ids.len());
    let mut out = Vec::new();
    for i in 0..n {
        let cab = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        if cab.is_empty() {
            continue;
        }
        let Some(adv) = ids
            .get(i)
            .and_then(|s| parse_i64_cell(s))
            .filter(|&x| x > 0)
        else {
            continue;
        };
        let nm = nms
            .get(i)
            .and_then(|s| parse_i64_cell(s))
            .filter(|&x| x > 0);
        out.push((normalize_cabinet_name(cab), adv, nm));
    }
    Ok(out)
}

/// Тройки с обязательным nm.
pub async fn load_cabinet_advert_nm_required(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    cab_range: &str,
    id_range: &str,
    nm_range: &str,
) -> Result<Vec<(String, i64, i64)>, String> {
    let rows =
        load_cabinet_advert_nm_rows(client, spreadsheet_id, sheet_name, cab_range, id_range, nm_range)
            .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(c, a, n)| n.map(|nm| (c, a, nm)))
        .collect())
}

/// Кабинет → уникальные nmId.
pub async fn load_cabinet_nm_map(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    cab_range: &str,
    nm_range: &str,
) -> Result<HashMap<String, Vec<i64>>, String> {
    let cols = read_first_cols(client, spreadsheet_id, sheet_name, &[cab_range, nm_range]).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    let n = cabs.len().max(nms.len());
    let mut map: HashMap<String, HashSet<i64>> = HashMap::new();
    for i in 0..n {
        let cab = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        if cab.is_empty() {
            continue;
        }
        let Some(nm) = nms.get(i).and_then(|s| parse_i64_cell(s)).filter(|&x| x > 0) else {
            continue;
        };
        map.entry(normalize_cabinet_name(cab))
            .or_default()
            .insert(nm);
    }
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            let mut xs: Vec<_> = v.into_iter().collect();
            xs.sort_unstable();
            (k, xs)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_with_sheet_quotes_and_escapes() {
        assert_eq!(a1_with_sheet("Лист1", "A2:A"), "'Лист1'!A2:A");
        assert_eq!(a1_with_sheet("O'Brien", "B:B"), "'O''Brien'!B:B");
    }

    #[test]
    fn normalize_cabinet_strips_api_key_prefix() {
        assert_eq!(normalize_cabinet_name("API_KEY_Shop"), "Shop");
        assert_eq!(normalize_cabinet_name("  Shop  "), "Shop");
    }

    #[test]
    fn parse_i64_cell_accepts_spaces_and_comma() {
        assert_eq!(parse_i64_cell("1 234"), Some(1234));
        assert_eq!(parse_i64_cell("12,5"), Some(12));
        assert_eq!(parse_i64_cell(""), None);
        assert_eq!(parse_i64_cell("abc"), None);
    }

    #[test]
    fn flatten_first_col_pads_empty() {
        let v = vec![vec!["a".into()], vec![], vec!["c".into(), "x".into()]];
        assert_eq!(flatten_first_col(&v), vec!["a", "", "c"]);
    }
}
