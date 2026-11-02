//! clear_and_write + расширение сетки ( WriteDataSmart / EnsureSheetCapacity).
//! Hot path: 429/quota backoff (`http::exchange_with_backoff`) + resume с уже записанного offset.

use super::http as google_http;
use super::sheets::{
    encode_range, looks_like_missing_sheet, quote_sheet, to_a1_column, truncate, GoogleSheetsClient,
    BATCH_ROWS,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Serialize)]
struct UpdateValuesBody<'a> {
    range: &'a str,
    #[serde(rename = "majorDimension")]
    major_dimension: &'a str,
    values: &'a [Vec<String>],
}

impl GoogleSheetsClient {
    /// WriteDataSmart: `clear_only_written_range=true` — только колонки данных;
    /// `false` — clear всего листа (как ClearSheetAsync). Затем Values.Update батчами.
    pub async fn clear_and_write(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rows: Vec<Vec<String>>,
        clear_only_written_range: bool,
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
        let last_col = to_a1_column(cols);

        let clear_a1 = if clear_only_written_range {
            // Used-range: не clear целых колонок A:Z (дорого). Длина A без парса строк.
            let old_rows = self
                .read_values_row_count(sid, &format!("{}!A:A", quote_sheet(sheet)))
                .await
                .unwrap_or(0);
            let clear_rows = rows.len().max(old_rows).max(1);
            format!("{}!A1:{}{clear_rows}", quote_sheet(sheet), last_col)
        } else {
            // Весь лист ( ClearSheetAsync): range = 'SheetName'
            quote_sheet(sheet)
        };
        match self.clear_range(sid, &clear_a1).await {
            Ok(()) => {}
            Err(e) => {
                if looks_like_missing_sheet(&e) {
                    self.add_sheet(sid, sheet).await?;
                } else {
                    return Err(format!("clear {clear_a1}: {e}"));
                }
            }
        }

        if rows.is_empty() {
            return Ok(0);
        }

        // Иначе Update A2001:… → HTTP 400 «exceeds grid limits».
        self.ensure_sheet_capacity(sid, sheet, rows.len(), cols)
            .await?;

        let mut written = 0usize;
        let mut i = 0usize;
        while i < rows.len() {
            let end = (i + BATCH_ROWS).min(rows.len());
            let chunk = &rows[i..end];
            let start_row = i + 1;
            let end_row = end;
            let range = format!("{}!A{start_row}:{last_col}{end_row}", quote_sheet(sheet));
            // Partial resume: не переписываем уже успешные батчи; при 429 — ещё пауза и тот же offset.
            let mut outer = 0u32;
            loop {
                match self.update_values(sid, &range, chunk).await {
                    Ok(()) => {
                        written = end;
                        i = end;
                        break;
                    }
                    Err(e) if google_http::error_is_rate_or_quota(&e) && outer < 2 => {
                        outer += 1;
                        sleep(Duration::from_millis(2_000u64 << outer)).await;
                    }
                    Err(e) => {
                        return Err(format!(
                            "update {range} (уже записано строк: {written}): {e}"
                        ));
                    }
                }
            }
        }
        Ok(written)
    }

    async fn ensure_sheet_capacity(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rows: usize,
        cols: usize,
    ) -> Result<(), String> {
        let need_rows = rows.max(1);
        let need_cols = cols.max(1);
        let (sheet_id, cur_rows, cur_cols) =
            self.sheet_grid_meta(spreadsheet_id, sheet_name).await?;
        if cur_rows >= need_rows && cur_cols >= need_cols {
            return Ok(());
        }
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

    async fn sheet_grid_meta(
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
}
