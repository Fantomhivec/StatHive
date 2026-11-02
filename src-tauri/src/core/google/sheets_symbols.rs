//! Окраска латиницы/кириллицы (symbols_check) через textFormatRuns.

use super::http as google_http;
use super::sheets::{encode_range, quote_sheet, truncate, GoogleSheetsClient};
use serde_json::{json, Value};

/// EN → красный, RU → синий; только user-entered string (не формулы).
pub async fn apply_symbols_check_coloring(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    a1_range: &str,
    on_log: &(dyn Fn(String) + Send + Sync),
) -> Result<(), String> {
    let sid = spreadsheet_id.trim();
    let sheet = sheet_name.trim();
    let range = a1_range.trim();
    if sid.is_empty() || sheet.is_empty() || range.is_empty() {
        return Err("Укажите таблицу, лист и диапазон".into());
    }

    let sheet_id = sheet_id_by_title(client, sid, sheet).await?;
    let full_a1 = format!("{}!{range}", quote_sheet(sheet));

    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{sid}?includeGridData=true&ranges={}&fields={}",
        encode_range(&full_a1),
        encode_range("sheets(properties(sheetId),data(rowData(values(userEnteredValue))))")
    );
    let token = client.access_token.clone();
    let http = client.http.clone();
    let (status, text) = google_http::exchange_with_backoff(|| {
        http.get(&url).bearer_auth(&token).send()
    })
    .await?;
    if !status.is_success() {
        return Err(format!(
            "Sheets grid get HTTP {status}: {}",
            truncate(&text, 200)
        ));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let row_data = v
        .pointer("/sheets/0/data/0/rowData")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    if row_data.is_empty() {
        on_log("Нет данных в диапазоне".into());
        return Ok(());
    }

    let (start_row, start_col) = parse_a1_start(range);
    let total_rows = row_data.len();
    let col_count = row_data
        .iter()
        .map(|r| {
            r.get("values")
                .and_then(|x| x.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    if col_count == 0 {
        return Ok(());
    }

    const ROWS_PER_BATCH: usize = 1000;
    let batches = total_rows.div_ceil(ROWS_PER_BATCH);
    on_log(format!(
        "Найдено строк: {total_rows}. Батчей: {batches}, по {ROWS_PER_BATCH}."
    ));

    for batch_i in 0..batches {
        let batch_start = batch_i * ROWS_PER_BATCH;
        let batch_end = (batch_start + ROWS_PER_BATCH).min(total_rows);
        let mut requests = Vec::new();

        for r in batch_start..batch_end {
            let cells = row_data[r]
                .get("values")
                .and_then(|x| x.as_array())
                .cloned()
                .unwrap_or_default();
            if cells.is_empty() {
                continue;
            }
            let eligible = |col: usize| -> Option<String> {
                let cell = cells.get(col)?;
                let ue = cell.get("userEnteredValue")?;
                if ue.get("formulaValue").is_some() {
                    return None;
                }
                ue.get("stringValue")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            };

            let mut c = 0usize;
            while c < col_count {
                let Some(first) = eligible(c) else {
                    c += 1;
                    continue;
                };
                let seg_start = c;
                let mut strings = vec![first];
                c += 1;
                while c < col_count {
                    let Some(s) = eligible(c) else {
                        break;
                    };
                    strings.push(s);
                    c += 1;
                }
                let values: Vec<Value> = strings
                    .iter()
                    .map(|s| {
                        json!({
                            "textFormatRuns": build_symbol_runs(s)
                        })
                    })
                    .collect();
                requests.push(json!({
                    "updateCells": {
                        "range": {
                            "sheetId": sheet_id,
                            "startRowIndex": start_row + r as i64,
                            "endRowIndex": start_row + r as i64 + 1,
                            "startColumnIndex": start_col + seg_start as i64,
                            "endColumnIndex": start_col + seg_start as i64 + values.len() as i64
                        },
                        "rows": [{ "values": values }],
                        "fields": "textFormatRuns"
                    }
                }));
            }
        }

        if requests.is_empty() {
            on_log(format!(
                "Пакет {}/{batches}: нет строк для обновления",
                batch_i + 1
            ));
            continue;
        }

        let body = json!({ "requests": requests });
        let bu_url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{sid}:batchUpdate"
        );
        let token = client.access_token.clone();
        let http = client.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.post(&bu_url)
                .bearer_auth(&token)
                .json(&body)
                .send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!(
                "Sheets batchUpdate symbols HTTP {status}: {}",
                truncate(&text, 220)
            ));
        }
        on_log(format!(
            "Пакет {}/{batches} выполнен. Строк: {batch_end}/{total_rows}.",
            batch_i + 1
        ));
    }

    Ok(())
}

async fn sheet_id_by_title(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
) -> Result<i64, String> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(sheetId,title)"
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
    if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
        for sh in sheets {
            let title = sh
                .pointer("/properties/title")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if title.eq_ignore_ascii_case(sheet_name) {
                if let Some(id) = sh.pointer("/properties/sheetId").and_then(|x| x.as_i64()) {
                    return Ok(id);
                }
            }
        }
    }
    Err(format!("Лист «{sheet_name}» не найден"))
}

/// Старт диапазона A1 → (row0, col0). Упрощённо: A1 / A1:Z10 / A:Z / A1:A.
fn parse_a1_start(a1: &str) -> (i64, i64) {
    let start = a1.split_once(':').map(|(a, _)| a).unwrap_or(a1).trim();
    let mut col = 0i64;
    let mut row = 0i64;
    let mut letters = String::new();
    let mut digits = String::new();
    for ch in start.chars() {
        if ch.is_ascii_alphabetic() {
            letters.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            digits.push(ch);
        }
    }
    for ch in letters.chars() {
        col = col * 26 + (ch as u8 - b'A' + 1) as i64;
    }
    if col > 0 {
        col -= 1;
    }
    if !digits.is_empty() {
        row = digits.parse::<i64>().unwrap_or(1).saturating_sub(1);
    }
    (row, col)
}

fn build_symbol_runs(text: &str) -> Vec<Value> {
    if text.is_empty() {
        return Vec::new();
    }
    fn classify(ch: char) -> u8 {
        if ch.is_ascii_alphabetic() {
            1
        } else if ('\u{0410}'..='\u{044F}').contains(&ch) || ch == '\u{0401}' || ch == '\u{0451}' {
            2
        } else {
            0
        }
    }
    let red = json!({ "red": 1.0, "green": 0.0, "blue": 0.0 });
    let blue = json!({ "red": 0.0, "green": 0.0, "blue": 1.0 });
    let mut runs = vec![json!({ "startIndex": 0, "format": {} })];
    let chars: Vec<char> = text.chars().collect();
    let mut prev = classify(chars[0]);
    let mut emit = |start: usize, cls: u8| {
        let format = match cls {
            1 => json!({ "foregroundColor": red }),
            2 => json!({ "foregroundColor": blue }),
            _ => json!({}),
        };
        runs.push(json!({ "startIndex": start, "format": format }));
    };
    emit(0, prev);
    for (i, &ch) in chars.iter().enumerate().skip(1) {
        let cls = classify(ch);
        if cls != prev {
            prev = cls;
            emit(i, cls);
        }
    }
    // dedupe by startIndex keep last
    let mut map = std::collections::BTreeMap::new();
    for r in runs {
        let idx = r.get("startIndex").and_then(|x| x.as_u64()).unwrap_or(0);
        map.insert(idx, r);
    }
    map.into_values().collect()
}
