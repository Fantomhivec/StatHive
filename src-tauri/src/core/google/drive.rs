//! Google Drive helpers for calculator spreadsheet creation (user OAuth).

use super::sheets::GoogleSheetsClient;
use serde_json::{json, Value};

impl GoogleSheetsClient {
    /// Найти или создать папку на Drive пользователя.
    pub async fn ensure_folder(&self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Имя папки пусто".into());
        }
        let q = format!(
            "name = '{}' and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
            name.replace('\'', "\\'")
        );
        let url = format!(
            "https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name)&pageSize=5",
            urlencoding::encode(&q)
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
        if let Some(files) = v.get("files").and_then(|x| x.as_array()) {
            if let Some(id) = files
                .first()
                .and_then(|f| f.get("id"))
                .and_then(|x| x.as_str())
            {
                return Ok(id.to_string());
            }
        }

        let body = json!({
            "name": name,
            "mimeType": "application/vnd.google-apps.folder"
        });
        let res = self
            .http
            .post("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("create folder HTTP {status}: {}", truncate(&text, 200)));
        }
        let created: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        created
            .get("id")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Drive: нет id папки".into())
    }

    /// Создать Google Sheet в папке с нужным именем первого листа (без лишнего Sheet1/Лист1).
    /// Spreadsheets.Create с title листа → Drive addParents.
    pub async fn create_spreadsheet_in_folder(
        &self,
        title: &str,
        folder_id: &str,
        first_sheet: &str,
    ) -> Result<String, String> {
        let sheet_title = {
            let t = first_sheet.trim();
            if t.is_empty() {
                "Лист1"
            } else {
                t
            }
        };
        let body = json!({
            "properties": { "title": title.trim() },
            "sheets": [{
                "properties": {
                    "title": sheet_title,
                    "gridProperties": { "rowCount": 1000, "columnCount": 26 }
                }
            }]
        });
        let res = self
            .http
            .post("https://sheets.googleapis.com/v4/spreadsheets?fields=spreadsheetId")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("create spreadsheet HTTP {status}: {}", truncate(&text, 240)));
        }
        let created: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let sid = created
            .get("spreadsheetId")
            .and_then(|x| x.as_str())
            .ok_or_else(|| "Sheets: нет spreadsheetId".to_string())?
            .to_string();

        // Переместить в папку калькулятора.
        let folder = folder_id.trim();
        if !folder.is_empty() {
            let url = format!(
                "https://www.googleapis.com/drive/v3/files/{sid}?addParents={folder}&fields=id,parents"
            );
            let res = self
                .http
                .patch(&url)
                .bearer_auth(&self.access_token)
                .json(&json!({}))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            if !status.is_success() {
                let err = res.text().await.unwrap_or_default();
                return Err(format!(
                    "move to folder HTTP {status}: {}",
                    truncate(&err, 200)
                ));
            }
        }

        // На случай старого пути Drive-mime: переименовать Sheet1/Лист1, не плодить листы.
        self.ensure_sheet_exists_or_rename(&sid, sheet_title, "Sheet1")
            .await?;
        Ok(sid)
    }

    /// Создать spreadsheet в корне Drive пользователя (workspace / без папки).
    pub async fn create_spreadsheet(&self, title: &str, first_sheet: &str) -> Result<String, String> {
        let sheet_title = {
            let t = first_sheet.trim();
            if t.is_empty() {
                "Лист1"
            } else {
                t
            }
        };
        let body = json!({
            "properties": { "title": title.trim() },
            "sheets": [{
                "properties": {
                    "title": sheet_title,
                    "gridProperties": { "rowCount": 1000, "columnCount": 26 }
                }
            }]
        });
        let res = self
            .http
            .post("https://sheets.googleapis.com/v4/spreadsheets?fields=spreadsheetId")
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "create spreadsheet HTTP {status}: {}",
                truncate(&text, 240)
            ));
        }
        let created: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        created
            .get("spreadsheetId")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Sheets: нет spreadsheetId".to_string())
    }

    /// Writer на файл (без email-уведомления — для Service Account).
    pub async fn share_writer_silent(&self, file_id: &str, email: &str) -> Result<(), String> {
        self.share_role_ex(file_id, email, "writer", false).await
    }

    /// Reader на файл (без уведомления) — участник калькулятора.
    pub async fn share_reader_silent(&self, file_id: &str, email: &str) -> Result<(), String> {
        self.share_role_ex(file_id, email, "reader", false).await
    }

    /// Снять доступ user по email (ротация SA). Нет permission — Ok.
    pub async fn remove_user_access(&self, file_id: &str, email: &str) -> Result<(), String> {
        let id = file_id.trim();
        let email = email.trim();
        if id.is_empty() || email.is_empty() {
            return Ok(());
        }
        let list_url = format!(
            "https://www.googleapis.com/drive/v3/files/{id}/permissions?fields=permissions(id,emailAddress,type)&pageSize=100"
        );
        let res = self
            .http
            .get(&list_url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "Drive permissions list HTTP {status}: {}",
                truncate(&text, 200)
            ));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let Some(perms) = v.get("permissions").and_then(|x| x.as_array()) else {
            return Ok(());
        };
        for p in perms {
            let typ = p.get("type").and_then(|x| x.as_str()).unwrap_or("");
            let addr = p
                .get("emailAddress")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if !typ.eq_ignore_ascii_case("user") || !addr.eq_ignore_ascii_case(email) {
                continue;
            }
            let Some(pid) = p.get("id").and_then(|x| x.as_str()) else {
                continue;
            };
            let del_url =
                format!("https://www.googleapis.com/drive/v3/files/{id}/permissions/{pid}");
            let del = self
                .http
                .delete(&del_url)
                .bearer_auth(&self.access_token)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let st = del.status();
            if !st.is_success() && st.as_u16() != 404 {
                let err = del.text().await.unwrap_or_default();
                return Err(format!(
                    "Drive permission delete HTTP {st}: {}",
                    truncate(&err, 200)
                ));
            }
        }
        Ok(())
    }

    async fn share_role_ex(
        &self,
        file_id: &str,
        email: &str,
        role: &str,
        send_notification: bool,
    ) -> Result<(), String> {
        let id = file_id.trim();
        let email = email.trim();
        if id.is_empty() || email.is_empty() {
            return Err("file_id и email обязательны".into());
        }
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{id}/permissions?sendNotificationEmail={send_notification}"
        );
        let body = json!({
            "type": "user",
            "role": role,
            "emailAddress": email
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
        if status.is_success() {
            return Ok(());
        }
        let err = res.text().await.unwrap_or_default();
        // Уже есть доступ — не ошибка.
        if status.as_u16() == 400 && err.contains("already") {
            return Ok(());
        }
        Err(format!(
            "Drive share HTTP {status}: {}",
            truncate(&err, 240)
        ))
    }

    /// Удалить файл/таблицу на Drive. 404 — уже нет, не ошибка (DeleteSpreadsheetAsync).
    pub async fn delete_spreadsheet(&self, file_id: &str) -> Result<(), String> {
        let id = file_id.trim();
        if id.is_empty() {
            return Ok(());
        }
        let url = format!("https://www.googleapis.com/drive/v3/files/{id}");
        let res = self
            .http
            .delete(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if status.is_success() || status.as_u16() == 404 {
            return Ok(());
        }
        let err = res.text().await.unwrap_or_default();
        Err(format!(
            "delete file HTTP {status}: {}",
            truncate(&err, 200)
        ))
    }

    /// Переименовать файл на Drive (название spreadsheet / команды).
    pub async fn rename_drive_file(&self, file_id: &str, new_name: &str) -> Result<(), String> {
        let id = file_id.trim();
        let name = new_name.trim();
        if id.is_empty() {
            return Err("Пустой id файла Drive".into());
        }
        if name.is_empty() {
            return Err("Пустое имя файла".into());
        }
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?fields=id,name",
            urlencoding::encode(id)
        );
        let res = self
            .http
            .patch(&url)
            .bearer_auth(&self.access_token)
            .json(&json!({ "name": name }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if !status.is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(format!(
                "rename file HTTP {status}: {}",
                truncate(&err, 200)
            ));
        }
        Ok(())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}
