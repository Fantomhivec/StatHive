// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! clear_and_write + расширение сетки ( WriteDataSmart / EnsureSheetCapacity).
//! Hot path: 429/quota backoff (`http::exchange_with_backoff`) + resume с уже записанного offset.

use super::super::http as google_http;
use super::service::SheetClearScope;
use super::{
    encode_range, looks_like_missing_sheet, quote_sheet, to_a1_column, truncate, GoogleSheetsClient,
    BATCH_ROWS,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
struct UpdateValuesBody<'a> {
    range: &'a str,
    #[serde(rename = "majorDimension")]
    major_dimension: &'a str,
    values: &'a [Vec<String>],
}

/// Progress line after each Sheets chunk, e.g. `Записано 10000 из 120000 (8%)`.
pub(crate) fn format_sheets_write_progress(written: usize, total: usize) -> String {
    format_sheets_op_progress("Записано", written, total)
}

pub(crate) fn format_sheets_op_progress(verb: &str, done: usize, total: usize) -> String {
    let pct = if total == 0 {
        100
    } else {
        (done.saturating_mul(100)) / total
    };
    format!("{verb} {done} из {total} ({pct}%)")
}

impl GoogleSheetsClient {
    /// WriteDataSmart: clear per [`SheetClearScope`], then Values.Update in chunks.
    /// Prefer [`GoogleSheetsClient::write_sheet`].
    pub(crate) async fn clear_and_write(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rows: Vec<Vec<String>>,
        clear: SheetClearScope,
        mut on_progress: impl FnMut(String),
    ) -> Result<usize, String> {
        let sid = spreadsheet_id.trim();
        let sheet = sheet_name.trim();
        if sid.is_empty() || sheet.is_empty() {
            return Err("Укажите spreadsheetId и имя листа".into());
        }

        let cols = rows
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(1)
            .max(1);
        let total = rows.len();
        let quoted = quote_sheet(sheet);
        let a_col = format!("{quoted}!A:A");

        // Meta always; used rows only for DataColumns (WholeSheet clears title range).
        let meta_res = self.sheet_grid_meta(sid, sheet).await;
        let (sheet_id, cur_rows, cur_cols) = match meta_res {
            Ok(m) => m,
            Err(e) if looks_like_missing_sheet(&e) => {
                self.add_sheet(sid, sheet).await?;
                self.sheet_grid_meta(sid, sheet).await?
            }
            Err(e) => return Err(e),
        };

        match clear {
            SheetClearScope::WholeSheet => {
                on_progress(format!(
                    "Очистка листа (весь лист)…"
                ));
                self.clear_range(sid, &quoted)
                    .await
                    .map_err(|e| format!("clear sheet {quoted}: {e}"))?;
            }
            SheetClearScope::DataColumns => {
                // Used-range height (A:A), not grid.rowCount — fat grids were 20+s clears.
                let used_rows = self
                    .read_values_row_count(sid, &a_col)
                    .await
                    .unwrap_or(0);
                let clear_cols = cols;
                let clear_rows = total.max(used_rows).max(1);
                let clear_a1 = format!("{quoted}!A1:{}{clear_rows}", to_a1_column(clear_cols));
                on_progress(format!(
                    "Очистка листа ({clear_rows} строк × {clear_cols} кол.)…"
                ));
                self.clear_range(sid, &clear_a1)
                    .await
                    .map_err(|e| format!("clear {clear_a1}: {e}"))?;
            }
        }

        if rows.is_empty() {
            return Ok(0);
        }

        let last_col = to_a1_column(cols);
        self.ensure_sheet_capacity_with_meta(
            sid,
            sheet_id,
            cur_rows,
            cur_cols,
            total,
            cols,
            &mut on_progress,
        )
        .await?;

        on_progress(format!("Запись в Sheets ({total} строк)…"));
        let mut written = 0usize;
        let mut i = 0usize;
        while i < total {
            self.check_cancelled()?;
            let end = (i + BATCH_ROWS).min(total);
            let chunk = &rows[i..end];
            let start_row = i + 1;
            let end_row = end;
            let range = format!("{quoted}!A{start_row}:{last_col}{end_row}");
            // 429/quota — только внутри exchange_with_backoff (без внешнего sleep-loop).
            self.update_values(sid, &range, chunk)
                .await
                .map_err(|e| format!("update {range} (уже записано строк: {written}): {e}"))?;
            written = end;
            i = end;
            on_progress(format_sheets_write_progress(written, total));
        }
        Ok(written)
    }

    pub(crate) async fn ensure_sheet_capacity_with_meta(
        &self,
        spreadsheet_id: &str,
        sheet_id: i64,
        cur_rows: usize,
        cur_cols: usize,
        need_rows: usize,
        need_cols: usize,
        on_progress: &mut impl FnMut(String),
    ) -> Result<(), String> {
        let need_rows = need_rows.max(1);
        let need_cols = need_cols.max(1);
        if cur_rows >= need_rows && cur_cols >= need_cols {
            return Ok(());
        }
        on_progress("Проверка размера сетки…".into());
        let new_rows = cur_rows.max(need_rows);
        let new_cols = cur_cols.max(need_cols);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
        );
        let body = json!({
            "requests": [{
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": sheet_id,
                        "gridProperties": {
                            "rowCount": new_rows,
                            "columnCount": new_cols
                        }
                    },
                    "fields": "gridProperties.rowCount,gridProperties.columnCount"
                }
            }]
        });
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.post(&url)
                .bearer_auth(&token)
                .json(&body)
                .send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!(
                "ensure capacity HTTP {status}: {}",
                truncate(&text, 220)
            ));
        }
        Ok(())
    }

    pub(crate) async fn sheet_grid_meta(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
    ) -> Result<(i64, usize, usize), String> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(sheetId,title,gridProperties(rowCount,columnCount))"
        );
        let v: Value = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
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
                if !title.eq_ignore_ascii_case(want) {
                    continue;
                }
                let sheet_id = sh
                    .pointer("/properties/sheetId")
                    .and_then(|x| x.as_i64())
                    .ok_or_else(|| format!("Нет sheetId для «{want}»"))?;
                let row_count = sh
                    .pointer("/properties/gridProperties/rowCount")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(1000) as usize;
                let col_count = sh
                    .pointer("/properties/gridProperties/columnCount")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(26) as usize;
                return Ok((sheet_id, row_count, col_count));
            }
        }
        Err(format!("Лист «{want}» не найден"))
    }

    pub(crate) async fn clear_range(&self, spreadsheet_id: &str, range: &str) -> Result<(), String> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:clear",
            spreadsheet_id,
            encode_range(range)
        );
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.post(&url)
                .bearer_auth(&token)
                .json(&json!({}))
                .send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {}", truncate(&text, 240)));
        }
        Ok(())
    }

    pub(crate) async fn update_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        rows: &[Vec<String>],
    ) -> Result<(), String> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=USER_ENTERED",
            spreadsheet_id,
            encode_range(range)
        );
        let body = UpdateValuesBody {
            range,
            major_dimension: "ROWS",
            values: rows,
        };
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.put(&url).bearer_auth(&token).json(&body).send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {}", truncate(&text, 240)));
        }
        Ok(())
    }

    /// Write `rows` starting at sheet row `start_row_1based`, in chunks of `BATCH_ROWS`.
    pub(crate) async fn update_values_chunked(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rows: &[Vec<String>],
        start_row_1based: usize,
        on_progress: &mut impl FnMut(String),
    ) -> Result<(), String> {
        if rows.is_empty() {
            return Ok(());
        }
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
        let last_col = to_a1_column(cols);
        let total = rows.len();
        let mut i = 0usize;
        while i < total {
            self.check_cancelled()?;
            let end = (i + BATCH_ROWS).min(total);
            let chunk = &rows[i..end];
            let start = start_row_1based + i;
            let end_row = start_row_1based + end - 1;
            let range = format!(
                "{}!A{start}:{last_col}{end_row}",
                quote_sheet(sheet_name)
            );
            self.update_values(spreadsheet_id, &range, chunk).await?;
            on_progress(format_sheets_write_progress(end, total));
            i = end;
        }
        Ok(())
    }
}
