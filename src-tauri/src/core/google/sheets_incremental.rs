//! Инкрементальная запись в Sheets (паритет IncrementalSheetWriter).

use super::sheets::{quote_sheet, to_a1_column, GoogleSheetsClient};
use super::sheets_normalize::{normalize_value, values_equal};
use serde_json::json;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Default)]
pub struct IncrementalWriteResult {
    pub updated_rows: usize,
    pub inserted_rows: usize,
    pub deleted_rows: usize,
}

impl GoogleSheetsClient {
    /// Синхронизация по 1–2 колонкам-ключам. Пустой лист → полная запись.
    pub async fn write_incremental(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rows: Vec<Vec<String>>,
        key_columns: &[usize],
        allow_delete: bool,
    ) -> Result<IncrementalWriteResult, String> {
        if rows.is_empty() {
            return Ok(IncrementalWriteResult::default());
        }
        if key_columns.is_empty() || key_columns.len() > 2 {
            return Err("Нужно 1 или 2 колонки-ключа".into());
        }
        let header = &rows[0];
        for &idx in key_columns {
            if idx >= header.len() {
                return Err(format!("Индекс ключа {idx} вне заголовка"));
            }
        }

        // Не тянем A:ZZ — только колонки заголовка (лишние пустые ZZ сильно тормозят I/O).
        let last_col = to_a1_column(header.len().max(1));
        let existing = self
            .read_values(
                spreadsheet_id,
                &format!("{}!A:{}", quote_sheet(sheet_name), last_col),
            )
            .await
            .unwrap_or_default();

        if existing.is_empty() {
            let n = self
                .clear_and_write(spreadsheet_id, sheet_name, rows, true)
                .await?;
            return Ok(IncrementalWriteResult {
                inserted_rows: n.saturating_sub(1),
                ..Default::default()
            });
        }

        // Ключ↔колонка: при смене порядка/набора заголовков выровнять строки листа под новый header.
        let mut existing = existing;
        if headers_differ(&existing[0], header) {
            let old_header = existing[0].clone();
            existing = remap_existing_to_header(&old_header, header, existing);
            let last_col = to_a1_column(header.len().max(1));
            let end_row = existing.len().max(1);
            ensure_capacity(
                self,
                spreadsheet_id,
                sheet_name,
                end_row,
                header.len().max(1),
            )
            .await?;
            let range = format!(
                "{}!A1:{}{end_row}",
                quote_sheet(sheet_name),
                last_col
            );
            self.update_values(spreadsheet_id, &range, &existing)
                .await?;
        }

        // Индексы вместо clone строк: (sheet_row_1based, index_in_existing).
        let mut existing_map: HashMap<String, VecDeque<(usize, usize)>> = HashMap::new();
        for (i, row) in existing.iter().enumerate().skip(1) {
            let key = normalized_key(row, key_columns);
            existing_map
                .entry(key)
                .or_default()
                .push_back((i + 1, i));
        }

        let mut desired_counts: HashMap<String, usize> = HashMap::new();
        for row in rows.iter().skip(1) {
            let key = normalized_key(row, key_columns);
            *desired_counts.entry(key).or_default() += 1;
        }

        // (sheet_row_1based, index_in_rows) / insert index_in_rows — clone только в API.
        let mut updates: Vec<(usize, usize)> = Vec::new();
        let mut insert_idxs: Vec<usize> = Vec::new();
        let mut stats = IncrementalWriteResult::default();

        for (src_idx, row) in rows.iter().enumerate().skip(1) {
            let key = normalized_key(row, key_columns);
            if let Some(queue) = existing_map.get_mut(&key) {
                if let Some((row_num, old_idx)) = queue.pop_front() {
                    if rows_differ(&existing[old_idx], row, header.len()) {
                        updates.push((row_num, src_idx));
                        stats.updated_rows += 1;
                    }
                    continue;
                }
            }
            insert_idxs.push(src_idx);
            stats.inserted_rows += 1;
        }

        updates.sort_by_key(|(n, _)| *n);
        let mut i = 0usize;
        while i < updates.len() {
            let start = updates[i].0;
            let mut batch: Vec<&[String]> = vec![&rows[updates[i].1]];
            let mut j = i + 1;
            while j < updates.len() && updates[j].0 == start + (j - i) as usize {
                batch.push(&rows[updates[j].1]);
                j += 1;
            }
            let end = start + batch.len() - 1;
            let last_col = to_a1_column(header.len().max(1));
            let range = format!(
                "{}!A{start}:{last_col}{end}",
                quote_sheet(sheet_name)
            );
            let owned: Vec<Vec<String>> = batch.iter().map(|r| r.to_vec()).collect();
            self.update_values(spreadsheet_id, &range, &owned).await?;
            i = j;
        }

        if !insert_idxs.is_empty() {
            let start_row = existing.len() + 1;
            let last_col = to_a1_column(header.len().max(1));
            let end_row = start_row + insert_idxs.len() - 1;
            ensure_capacity(
                self,
                spreadsheet_id,
                sheet_name,
                end_row,
                header.len().max(1),
            )
            .await?;
            let range = format!(
                "{}!A{start_row}:{last_col}{end_row}",
                quote_sheet(sheet_name)
            );
            let owned: Vec<Vec<String>> = insert_idxs.iter().map(|&i| rows[i].clone()).collect();
            self.update_values(spreadsheet_id, &range, &owned).await?;
        }

        if allow_delete {
            // Локальный снимок после update/insert — без второго полного GET.
            let mut sheet_now = existing;
            for &(row_num, src_idx) in &updates {
                let idx = row_num.saturating_sub(1);
                if idx < sheet_now.len() {
                    sheet_now[idx] = rows[src_idx].clone();
                }
            }
            for &src_idx in &insert_idxs {
                sheet_now.push(rows[src_idx].clone());
            }
            let mut to_delete: Vec<usize> = Vec::new();
            let mut seen: HashMap<String, usize> = HashMap::new();
            for (i, row) in sheet_now.iter().enumerate().skip(1) {
                let key = normalized_key(row, key_columns);
                let count = seen.get(&key).copied().unwrap_or(0);
                let desired = desired_counts.get(&key).copied().unwrap_or(0);
                if count >= desired {
                    to_delete.push(i + 1);
                }
                seen.insert(key, count + 1);
            }
            to_delete.sort_unstable_by(|a, b| b.cmp(a));
            if !to_delete.is_empty() {
                self.delete_sheet_rows(spreadsheet_id, sheet_name, &to_delete)
                    .await?;
                stats.deleted_rows = to_delete.len();
            }
        }

        Ok(stats)
    }

    pub(crate) async fn delete_sheet_rows(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        row_numbers_1based: &[usize],
    ) -> Result<(), String> {
        if row_numbers_1based.is_empty() {
            return Ok(());
        }
        let sheet_id = sheet_id_by_title(self, spreadsheet_id, sheet_name).await?;
        // Merge contiguous descending ranges for fewer requests.
        // Caller sorts descending; e.g. [10,9,8,5,4] → ranges [8..11) and [4..6) (0-based).
        let mut requests = Vec::new();
        let mut i = 0usize;
        while i < row_numbers_1based.len() {
            let end_1based = row_numbers_1based[i];
            let mut start_1based = end_1based;
            let mut j = i + 1;
            while j < row_numbers_1based.len()
                && row_numbers_1based[j] + 1 == start_1based
            {
                start_1based = row_numbers_1based[j];
                j += 1;
            }
            requests.push(json!({
                "deleteDimension": {
                    "range": {
                        "sheetId": sheet_id,
                        "dimension": "ROWS",
                        "startIndex": (start_1based as i64) - 1,
                        "endIndex": end_1based as i64
                    }
                }
            }));
            i = j;
        }
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
        );
        // Chunk requests (API limit).
        for chunk in requests.chunks(100) {
            let body = json!({ "requests": chunk });
            let res = self
                .http
                .post(&url)
                .bearer_auth(&self.access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            if !status.is_success() {
                let text = res.text().await.unwrap_or_default();
                return Err(format!("deleteDimension HTTP {status}: {text}"));
            }
        }
        Ok(())
    }
}

/// BuildNormalizedKey: каждая часть через NormalizeValue + `"||"` (пустые части не skip).
fn normalized_key(row: &[String], key_columns: &[usize]) -> String {
    let mut out = String::new();
    for &idx in key_columns {
        let v = row.get(idx).map(|s| s.as_str()).unwrap_or("");
        out.push_str(&normalize_value(v));
        out.push_str("||");
    }
    out
}

fn rows_differ(old: &[String], new: &[String], cols: usize) -> bool {
    for i in 0..cols {
        let a = old.get(i).map(|s| s.as_str()).unwrap_or("");
        let b = new.get(i).map(|s| s.as_str()).unwrap_or("");
        if !values_equal(a, b) {
            return true;
        }
    }
    false
}

fn headers_differ(old: &[String], new: &[String]) -> bool {
    let cols = old.len().max(new.len());
    rows_differ(old, new, cols)
}

/// Переставить ячейки строк листа под новый порядок заголовков (совпадение имён, без учёта регистра).
fn remap_existing_to_header(
    old_header: &[String],
    new_header: &[String],
    existing: Vec<Vec<String>>,
) -> Vec<Vec<String>> {
    if existing.is_empty() {
        return existing;
    }
    let mut old_index: HashMap<String, usize> = HashMap::new();
    for (i, h) in old_header.iter().enumerate() {
        let k = h.trim().to_ascii_lowercase();
        if !k.is_empty() {
            old_index.entry(k).or_insert(i);
        }
    }
    let mut out = Vec::with_capacity(existing.len());
    out.push(new_header.to_vec());
    for row in existing.into_iter().skip(1) {
        let mut mapped = vec![String::new(); new_header.len()];
        for (new_i, h) in new_header.iter().enumerate() {
            let k = h.trim().to_ascii_lowercase();
            if let Some(&old_i) = old_index.get(&k) {
                if let Some(v) = row.get(old_i) {
                    mapped[new_i] = v.clone();
                }
            }
        }
        out.push(mapped);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_reorders_by_header_names() {
        let old_h = vec!["A".into(), "B".into(), "C".into()];
        let new_h = vec!["C".into(), "A".into()];
        let existing = vec![
            old_h.clone(),
            vec!["1".into(), "2".into(), "3".into()],
            vec!["x".into(), "y".into(), "z".into()],
        ];
        let out = remap_existing_to_header(&old_h, &new_h, existing);
        assert_eq!(out[0], new_h);
        assert_eq!(out[1], vec!["3".to_string(), "1".to_string()]);
        assert_eq!(out[2], vec!["z".to_string(), "x".to_string()]);
    }

    #[test]
    fn remap_case_insensitive_and_missing() {
        let old_h = vec!["NmId".into(), "Name".into()];
        let new_h = vec!["name".into(), "price".into(), "nmid".into()];
        let existing = vec![old_h.clone(), vec!["10".into(), "foo".into()]];
        let out = remap_existing_to_header(&old_h, &new_h, existing);
        assert_eq!(
            out[1],
            vec!["foo".to_string(), String::new(), "10".to_string()]
        );
    }
}

async fn sheet_id_by_title(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
) -> Result<i64, String> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(sheetId,title)"
    );
    let v: serde_json::Value = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let want = sheet_name.trim();
    if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
        for sh in sheets {
            let title = sh
                .pointer("/properties/title")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if title.eq_ignore_ascii_case(want) {
                return sh
                    .pointer("/properties/sheetId")
                    .and_then(|x| x.as_i64())
                    .ok_or_else(|| format!("Нет sheetId для «{want}»"));
            }
        }
    }
    Err(format!("Лист «{want}» не найден"))
}

async fn ensure_capacity(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    need_rows: usize,
    need_cols: usize,
) -> Result<(), String> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(sheetId,title,gridProperties(rowCount,columnCount))"
    );
    let v: serde_json::Value = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let want = sheet_name.trim();
    let mut sheet_id = None;
    let mut cur_rows = 1000usize;
    let mut cur_cols = 26usize;
    if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
        for sh in sheets {
            let title = sh
                .pointer("/properties/title")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if !title.eq_ignore_ascii_case(want) {
                continue;
            }
            sheet_id = sh.pointer("/properties/sheetId").and_then(|x| x.as_i64());
            cur_rows = sh
                .pointer("/properties/gridProperties/rowCount")
                .and_then(|x| x.as_u64())
                .unwrap_or(1000) as usize;
            cur_cols = sh
                .pointer("/properties/gridProperties/columnCount")
                .and_then(|x| x.as_u64())
                .unwrap_or(26) as usize;
            break;
        }
    }
    let Some(sid) = sheet_id else {
        return Err(format!("Лист «{want}» не найден"));
    };
    if cur_rows >= need_rows && cur_cols >= need_cols {
        return Ok(());
    }
    let body = json!({
        "requests": [{
            "updateSheetProperties": {
                "properties": {
                    "sheetId": sid,
                    "gridProperties": {
                        "rowCount": cur_rows.max(need_rows),
                        "columnCount": cur_cols.max(need_cols)
                    }
                },
                "fields": "gridProperties.rowCount,gridProperties.columnCount"
            }
        }]
    });
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
    );
    let res = client
        .http
        .post(&url)
        .bearer_auth(&client.access_token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    if !status.is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("ensure capacity HTTP {status}: {text}"));
    }
    Ok(())
}
