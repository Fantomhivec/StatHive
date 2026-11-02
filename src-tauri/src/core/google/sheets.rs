//! Универсальный клиент Google Sheets (WriteDataSmart):
//! clearOnlyWrittenRange — clear только колонок данных (`A:I`), затем Values.Update.
//! Без `spreadsheets.get` и без clear всего листа (на больших книгах зависало).
//! Auth: user OAuth (скрипты/калькулятор) или Service Account (лист «Пользователи»).

use super::http as google_http;
use super::oauth;
use super::service_account;
use serde_json::{json, Value};

pub(crate) const BATCH_ROWS: usize = 2000;

#[derive(Clone)]
pub struct GoogleSheetsClient {
    pub(crate) access_token: String,
    pub(crate) http: reqwest::Client,
}

impl GoogleSheetsClient {
    /// Sheets/Drive — OAuth текущего пользователя.
    pub async fn from_user_oauth() -> Result<Self, String> {
        let access_token = oauth::user_access_token().await?;
        Self::from_access_token(access_token)
    }

    /// Sheets/Drive — Service Account (workspace / auth, без шаринга людям).
    pub async fn from_service_account() -> Result<Self, String> {
        let access_token = service_account::user_access_token().await?;
        Self::from_access_token(access_token)
    }

    fn from_access_token(access_token: String) -> Result<Self, String> {
        Ok(Self {
            access_token,
            http: google_http::client(),
        })
    }

    pub async fn test_connection(&self) -> Result<(), String> {
        let url = "https://sheets.googleapis.com/v4/spreadsheets/invalid_spreadsheet_id_for_probe";
        let res = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let code = res.status().as_u16();
        if code == 401 || code == 403 {
            return Err(format!("Нет доступа к Google Sheets API (HTTP {code})"));
        }
        Ok(())
    }

    pub async fn read_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> Result<Vec<Vec<String>>, String> {
        let url = values_url(spreadsheet_id, range, "get");
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.get(&url).bearer_auth(&token).send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!(
                "Sheets get HTTP {status}: {}",
                truncate(&text, 200)
            ));
        }
        // Прямой deserialize матрицы — без дерева всего JSON response.
        #[derive(serde::Deserialize)]
        struct ValuesResp {
            values: Option<Vec<Vec<Option<String>>>>,
        }
        let parsed: ValuesResp = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Ok(normalize_opt_matrix(parsed.values))
    }

    /// Длина колонки без аллокации `Vec<String>` на каждую ячейку (для clear used-range).
    pub async fn read_values_row_count(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> Result<usize, String> {
        let url = values_url(spreadsheet_id, range, "get");
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.get(&url).bearer_auth(&token).send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!(
                "Sheets get HTTP {status}: {}",
                truncate(&text, 200)
            ));
        }
        // Только длина values[] — строки как IgnoredAny (без String-ячеек).
        #[derive(serde::Deserialize)]
        struct RowCountResp {
            #[serde(default)]
            values: Vec<serde::de::IgnoredAny>,
        }
        let parsed: RowCountResp = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Ok(parsed.values.len())
    }

    /// Несколько диапазонов одним `values:batchGet` (быстрее, чем N отдельных get).
    pub async fn read_values_batch(
        &self,
        spreadsheet_id: &str,
        ranges: &[&str],
    ) -> Result<Vec<Vec<Vec<String>>>, String> {
        if ranges.is_empty() {
            return Ok(Vec::new());
        }
        if ranges.len() == 1 {
            return Ok(vec![self.read_values(spreadsheet_id, ranges[0]).await?]);
        }
        let sid = spreadsheet_id.trim();
        let mut url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{sid}/values:batchGet?valueRenderOption=FORMATTED_VALUE"
        );
        for r in ranges {
            url.push_str("&ranges=");
            url.push_str(&encode_range(r));
        }
        let token = self.access_token.clone();
        let http = self.http.clone();
        let (status, text) = google_http::exchange_with_backoff(|| {
            http.get(&url).bearer_auth(&token).send()
        })
        .await?;
        if !status.is_success() {
            return Err(format!(
                "Sheets batchGet HTTP {status}: {}",
                truncate(&text, 200)
            ));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(ranges.len());
        if let Some(vrs) = v.get("valueRanges").and_then(|x| x.as_array()) {
            for vr in vrs {
                out.push(parse_values_matrix(vr.get("values")));
            }
        }
        while out.len() < ranges.len() {
            out.push(Vec::new());
        }
        Ok(out)
    }

    /// Values.Update (USER_ENTERED) — ячейки/диапазоны (auth session, last login).
    pub async fn write_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), String> {
        self.update_values(spreadsheet_id, range, &rows).await
    }

    /// Имена листов — лёгкий get только `sheets.properties.title` (без полной сетки).
    pub async fn list_sheet_titles(&self, spreadsheet_id: &str) -> Result<Vec<String>, String> {
        Ok(self.list_spreadsheet_meta(spreadsheet_id).await?.1)
    }

    /// Заголовок документа + имена листов (`properties.title` + sheets).
    pub async fn list_spreadsheet_meta(
        &self,
        spreadsheet_id: &str,
    ) -> Result<(String, Vec<String>), String> {
        let sid = spreadsheet_id.trim();
        if sid.is_empty() {
            return Err("Укажите spreadsheetId".into());
        }
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{sid}?fields=properties.title,sheets.properties.title"
        );
        let token = self.access_token.clone();
        let http = self.http.clone();
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
        let doc_title = v
            .pointer("/properties/title")
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        let mut titles = Vec::new();
        if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
            for sh in sheets {
                if let Some(t) = sh
                    .pointer("/properties/title")
                    .and_then(|x| x.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    titles.push(t.to_string());
                }
            }
        }
        Ok((doc_title, titles))
    }

    pub async fn ensure_sheet_exists(&self, spreadsheet_id: &str, sheet_name: &str) -> Result<(), String> {
        let titles = self.list_sheet_titles(spreadsheet_id).await?;
        if titles
            .iter()
            .any(|t| t.eq_ignore_ascii_case(sheet_name.trim()))
        {
            return Ok(());
        }
        self.add_sheet(spreadsheet_id, sheet_name).await
    }

    pub async fn ensure_sheet_exists_or_rename(
        &self,
        spreadsheet_id: &str,
        sheet_name: &str,
        rename_from: &str,
    ) -> Result<(), String> {
        let titles = self.list_sheet_titles(spreadsheet_id).await?;
        let want = sheet_name.trim();
        if titles.iter().any(|t| t.eq_ignore_ascii_case(want)) {
            return Ok(());
        }
        // Drive/locale: «Sheet1» или «Лист1»; иначе единственный лист.
        let mut candidates: Vec<&str> = Vec::new();
        if !rename_from.trim().is_empty() {
            candidates.push(rename_from.trim());
        }
        for alt in ["Sheet1", "Лист1"] {
            if !candidates.iter().any(|c| c.eq_ignore_ascii_case(alt)) {
                candidates.push(alt);
            }
        }
        for from in &candidates {
            if let Some(title) = titles.iter().find(|t| t.eq_ignore_ascii_case(from)) {
                return self.rename_sheet(spreadsheet_id, title, want).await;
            }
        }
        if titles.len() == 1 {
            return self.rename_sheet(spreadsheet_id, &titles[0], want).await;
        }
        self.add_sheet(spreadsheet_id, want).await
    }

    async fn rename_sheet(
        &self,
        spreadsheet_id: &str,
        from_title: &str,
        to_title: &str,
    ) -> Result<(), String> {
        // Need sheetId — light get
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}?fields=sheets.properties(sheetId,title)"
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
        let mut sheet_id: Option<i64> = None;
        if let Some(sheets) = v.get("sheets").and_then(|x| x.as_array()) {
            for sh in sheets {
                let title = sh
                    .pointer("/properties/title")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if title.eq_ignore_ascii_case(from_title) {
                    sheet_id = sh.pointer("/properties/sheetId").and_then(|x| x.as_i64());
                    break;
                }
            }
        }
        let Some(sid) = sheet_id else {
            return self.add_sheet(spreadsheet_id, to_title).await;
        };
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
        );
        let body = json!({
            "requests": [{
                "updateSheetProperties": {
                    "properties": { "sheetId": sid, "title": to_title },
                    "fields": "title"
                }
            }]
        });
        let res = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("renameSheet HTTP {status}: {}", truncate(&text, 200)));
        }
        Ok(())
    }

    pub(crate) async fn add_sheet(&self, spreadsheet_id: &str, sheet_name: &str) -> Result<(), String> {
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
        );
        let body = json!({
            "requests": [{ "addSheet": { "properties": { "title": sheet_name } } }]
        });
        let res = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            // лист уже есть — ок
            if text.contains("already exists") {
                return Ok(());
            }
            return Err(format!("addSheet HTTP {status}: {}", truncate(&text, 200)));
        }
        Ok(())
    }
}

pub(crate) fn quote_sheet(sheet: &str) -> String {
    let escaped = sheet.replace('\'', "''");
    format!("'{escaped}'")
}

/// Google API: range в path должен быть percent-encoded (включая кавычки и кириллицу).
pub(crate) fn encode_range(range: &str) -> String {
    urlencoding::encode(range).into_owned()
}

fn values_url(spreadsheet_id: &str, range: &str, _op: &str) -> String {
    format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueRenderOption=FORMATTED_VALUE",
        spreadsheet_id.trim(),
        encode_range(range)
    )
}

pub(crate) fn looks_like_missing_sheet(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("unable to parse range")
        || e.contains("not found")
        || e.contains("unable to parse")
        || e.contains("invalid_argument")
}

fn normalize_opt_matrix(values: Option<Vec<Vec<Option<String>>>>) -> Vec<Vec<String>> {
    values
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.unwrap_or_default()).collect())
        .collect()
}

fn parse_values_matrix(v: Option<&Value>) -> Vec<Vec<String>> {
    let Some(arr) = v.and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .map(|row| {
            row.as_array()
                .map(|cells| {
                    cells
                        .iter()
                        .map(|c| match c {
                            Value::String(s) => s.clone(),
                            Value::Null => String::new(),
                            other => other.to_string().trim_matches('"').to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

pub(crate) fn to_a1_column(mut n: usize) -> String {
    if n == 0 {
        return "A".into();
    }
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_and_quote() {
        assert_eq!(to_a1_column(9), "I");
        assert_eq!(quote_sheet("Лист37"), "'Лист37'");
        assert!(encode_range("'Лист37'!A1:I10").contains("%"));
    }
}
