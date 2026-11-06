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

//! Drive list / permissions for вкладка «Все таблицы».

use super::sheets::GoogleSheetsClient;
use chrono::{DateTime, Local};
use serde_json::{json, Value};

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

fn role_display(role: &str) -> &str {
    match role.trim().to_ascii_lowercase().as_str() {
        "owner" => "владелец",
        "writer" => "редактор",
        "commenter" => "комментатор",
        "reader" => "читатель",
        _ => role,
    }
}

fn access_status(permissions: &[Value]) -> String {
    let link = permissions.iter().find(|p| {
        p.get("type")
            .and_then(|t| t.as_str())
            .is_some_and(|t| t.eq_ignore_ascii_case("anyone"))
    });
    match link {
        None => "Доступ закрыт".into(),
        Some(p) => {
            let role = p.get("role").and_then(|r| r.as_str()).unwrap_or("");
            format!("Доступ по ссылке ({})", role_display(role))
        }
    }
}

fn is_owner_me(file: &Value) -> bool {
    file.get("owners")
        .and_then(|o| o.as_array())
        .map(|owners| {
            owners.iter().any(|o| {
                o.get("me")
                    .and_then(|m| m.as_bool())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn format_created(raw: Option<&str>) -> String {
    let Some(s) = raw.map(str::trim).filter(|x| !x.is_empty()) else {
        return "Неизвестно".into();
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt
            .with_timezone(&Local)
            .format("%d.%m.%Y %H:%M")
            .to_string();
    }
    s.to_string()
}

impl GoogleSheetsClient {
    /// Все доступные spreadsheets (не Shared Drive corpora).
    pub async fn list_accessible_spreadsheets(&self) -> Result<Vec<Value>, String> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        let q = "mimeType='application/vnd.google-apps.spreadsheet' and trashed=false";
        let fields =
            "nextPageToken,files(id,name,owners,permissions,createdTime,modifiedTime)";
        loop {
            let mut url = format!(
                "https://www.googleapis.com/drive/v3/files?q={}&fields={}&pageSize=1000&orderBy=createdTime%20desc",
                urlencoding::encode(q),
                urlencoding::encode(fields)
            );
            if let Some(tok) = &page_token {
                url.push_str("&pageToken=");
                url.push_str(&urlencoding::encode(tok));
            }
            let res = self
                .http
                .get(&url)
                .bearer_auth(&self.access_token)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(format!(
                    "Drive files.list HTTP {status}: {}",
                    truncate(&text, 240)
                ));
            }
            let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            if let Some(files) = v.get("files").and_then(|f| f.as_array()) {
                all.extend(files.iter().cloned());
            }
            page_token = v
                .get("nextPageToken")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());
            if page_token.is_none() {
                break;
            }
        }
        Ok(all)
    }

    /// Owned или shared-only → строки для UI.
    pub fn map_all_tables_rows(files: Vec<Value>, shared_only: bool) -> Vec<Value> {
        let mut rows: Vec<(i64, Value)> = files
            .into_iter()
            .filter(|f| {
                let owner = is_owner_me(f);
                if shared_only {
                    !owner
                } else {
                    owner
                }
            })
            .map(|f| {
                let created_raw = f.get("createdTime").and_then(|c| c.as_str());
                let sort_key = created_raw
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0);
                let perms = f
                    .get("permissions")
                    .and_then(|p| p.as_array())
                    .cloned()
                    .unwrap_or_default();
                let users_count = perms
                    .iter()
                    .filter(|p| {
                        let t = p.get("type").and_then(|x| x.as_str()).unwrap_or("");
                        t.eq_ignore_ascii_case("user") || t.eq_ignore_ascii_case("group")
                    })
                    .count();
                let id = f
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = f
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("Без названия")
                    .to_string();
                let url = if id.is_empty() {
                    Value::Null
                } else {
                    json!(format!("https://docs.google.com/spreadsheets/d/{id}"))
                };
                let row = json!({
                    "id": id,
                    "name": name,
                    "url": url,
                    // Дата создания Google Sheet (не modifiedTime).
                    "created": format_created(created_raw),
                    "createdTs": sort_key,
                    "accessStatus": access_status(&perms),
                    "usersCount": users_count,
                    "isOwner": is_owner_me(&f),
                });
                (sort_key, row)
            })
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        rows.into_iter().map(|(_, r)| r).collect()
    }

    /// Permissions файла для диалога «Пользователи».
    pub async fn list_file_permissions_detail(
        &self,
        file_id: &str,
    ) -> Result<Vec<Value>, String> {
        let id = file_id.trim();
        if id.is_empty() {
            return Err("Пустой id таблицы".into());
        }
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/permissions?fields=permissions(id,emailAddress,role,type,displayName)&pageSize=100",
            urlencoding::encode(id)
        );
        let res = self
            .http
            .get(&url)
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
        let perms = v
            .get("permissions")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(perms
            .into_iter()
            .map(|p| {
                let pid = p.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let email = p
                    .get("emailAddress")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let role = p
                    .get("role")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let typ = p
                    .get("type")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let display = p
                    .get("displayName")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        if !email.is_empty() {
                            email.clone()
                        } else {
                            typ.clone()
                        }
                    });
                let can_remove =
                    !pid.is_empty() && !role.eq_ignore_ascii_case("owner");
                json!({
                    "id": pid,
                    "email": email,
                    "role": role,
                    "type": typ,
                    "displayName": display,
                    "canRemove": can_remove,
                })
            })
            .collect())
    }

    pub async fn remove_permission(
        &self,
        file_id: &str,
        permission_id: &str,
    ) -> Result<(), String> {
        let id = file_id.trim();
        let pid = permission_id.trim();
        if id.is_empty() || pid.is_empty() {
            return Err("file_id и permission_id обязательны".into());
        }
        let url = format!(
            "https://www.googleapis.com/drive/v3/files/{}/permissions/{}",
            urlencoding::encode(id),
            urlencoding::encode(pid)
        );
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
            "Drive permission delete HTTP {status}: {}",
            truncate(&err, 200)
        ))
    }
}
