//! Создание / ротация Service Account в GCP-проекте OAuth Desktop.
//! Доступ к данным — только Drive share на конкретную spreadsheet.

use crate::core::google_oauth;
use crate::core::i18n::{t, tf};
use crate::core::oauth_config;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Первый раз: стабильный account id по spreadsheet (переиспользуем SA, новый ключ если уже есть).
pub async fn create_key_json_for_workspace(spreadsheet_id: &str) -> Result<String, String> {
    provision(spreadsheet_id, false, None).await
}

/// Компрометация: новый SA + ключ. Старый email передаётся для удаления в GCP (Drive — снаружи).
pub async fn rotate_key_json_for_workspace(
    spreadsheet_id: &str,
    old_email: Option<&str>,
) -> Result<String, String> {
    provision(spreadsheet_id, true, old_email).await
}

async fn provision(
    spreadsheet_id: &str,
    rotate: bool,
    old_email: Option<&str>,
) -> Result<String, String> {
    let project =
        oauth_config::project_id().ok_or_else(|| t("errors.auth.oauthNoProjectId"))?;
    let sid = spreadsheet_id.trim();
    if sid.is_empty() {
        return Err(t("errors.auth.workspaceNotConfigured"));
    }

    let access_token = google_oauth::authorize_interactive_cloud().await?;
    let http = super::http::client();

    ensure_api_enabled(&http, &access_token, &project, "iam.googleapis.com").await?;
    ensure_api_enabled(&http, &access_token, &project, "sheets.googleapis.com").await?;

    let (email, resource_name) =
        ensure_service_account(&http, &access_token, &project, sid, rotate).await?;

    let key_json =
        create_service_account_key_json(&http, &access_token, &resource_name).await?;

    if rotate {
        if let Some(old) = old_email.map(str::trim).filter(|s| !s.is_empty()) {
            if !old.eq_ignore_ascii_case(&email) {
                let _ = delete_service_account(&http, &access_token, &project, old).await;
            }
        }
    }

    Ok(key_json)
}

/// Создать SA (при rotate — новый id; при create — стабильный / alreadyExists → reuse).
async fn ensure_service_account(
    http: &reqwest::Client,
    token: &str,
    project: &str,
    spreadsheet_id: &str,
    rotate: bool,
) -> Result<(String, String), String> {
    for attempt in 0..4u32 {
        let account_id = if rotate {
            account_id_rotate(spreadsheet_id, attempt)
        } else {
            account_id_stable(spreadsheet_id)
        };
        match create_service_account(http, token, project, &account_id).await {
            Ok(pair) => return Ok(pair),
            Err(e) if e.contains("alreadyExists") || e.contains("ALREADY_EXISTS") => {
                if rotate {
                    continue;
                }
                let email = format!("{account_id}@{project}.iam.gserviceaccount.com");
                let name = sa_resource_name(project, &email);
                return Ok((email, name));
            }
            Err(e) => return Err(e),
        }
    }
    Err(t("errors.auth.saCreateIdCollision"))
}

fn sa_resource_name(project: &str, sa_email: &str) -> String {
    // `@` в path без %40 ломает URL (userinfo) → 404 на create key.
    format!(
        "projects/{project}/serviceAccounts/{}",
        urlencoding::encode(sa_email.trim())
    )
}

fn account_id_stable(spreadsheet_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(spreadsheet_id.as_bytes());
    let hex = hex::encode(hasher.finalize());
    sanitize_account_id(&format!("ur-{}", &hex[..10]))
}

fn account_id_rotate(spreadsheet_id: &str, attempt: u32) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(spreadsheet_id.as_bytes());
    hasher.update(ts.to_be_bytes());
    hasher.update(attempt.to_be_bytes());
    let hex = hex::encode(hasher.finalize());
    sanitize_account_id(&format!("ur{}", &hex[..12]))
}

fn sanitize_account_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
                c
            } else if c.is_ascii_uppercase() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

async fn ensure_api_enabled(
    http: &reqwest::Client,
    token: &str,
    project: &str,
    service: &str,
) -> Result<(), String> {
    let url = format!(
        "https://serviceusage.googleapis.com/v1/projects/{project}/services/{service}:enable"
    );
    let res = http
        .post(&url)
        .bearer_auth(token)
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    if status.is_success() || status.as_u16() == 409 {
        return Ok(());
    }
    let err = res.text().await.unwrap_or_default();
    if status.as_u16() == 400
        && (err.contains("already enabled")
            || err.contains("Already opened")
            || err.contains("SERVICE_CONFIG"))
    {
        return Ok(());
    }
    Err(tf(
        "errors.auth.saEnableApiFailed",
        &[("service", service), ("error", &truncate(&err, 200))],
    ))
}

async fn create_service_account(
    http: &reqwest::Client,
    token: &str,
    project: &str,
    account_id: &str,
) -> Result<(String, String), String> {
    let url = format!("https://iam.googleapis.com/v1/projects/{project}/serviceAccounts");
    let body = json!({
        "accountId": account_id,
        "serviceAccount": {
            "displayName": "StatHive workspace auth",
            "description": "Sheets access for Users sheet only (shared per-file via Drive)"
        }
    });
    let res = http
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("IAM create SA HTTP {status}: {}", truncate(&text, 280)));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let email = v
        .get("email")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "IAM create SA: нет email".to_string())?
        .to_string();
    // Предпочитаем name из ответа (uniqueId) — без `@` в path.
    let resource_name = v
        .get("name")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| sa_resource_name(project, &email));
    Ok((email, resource_name))
}

async fn create_service_account_key_json(
    http: &reqwest::Client,
    token: &str,
    resource_name: &str,
) -> Result<String, String> {
    let url = format!("https://iam.googleapis.com/v1/{resource_name}/keys");
    let body = json!({
        "privateKeyType": "TYPE_GOOGLE_CREDENTIALS_FILE",
        "keyAlgorithm": "KEY_ALG_RSA_2048"
    });
    let mut last_err = String::new();
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt))).await;
        }
        let res = http
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if status.is_success() {
            let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let b64 = v
                .get("privateKeyData")
                .and_then(|x| x.as_str())
                .ok_or_else(|| "IAM create key: нет privateKeyData".to_string())?;
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| format!("IAM key base64: {e}"))?;
            return String::from_utf8(bytes).map_err(|e| format!("IAM key UTF-8: {e}"));
        }
        last_err = format!(
            "IAM create key HTTP {status}: {}",
            truncate(&text, 280)
        );
        if status.as_u16() != 404 {
            break;
        }
    }
    Err(last_err)
}

async fn delete_service_account(
    http: &reqwest::Client,
    token: &str,
    project: &str,
    sa_email: &str,
) -> Result<(), String> {
    let url = format!("https://iam.googleapis.com/v1/{}", sa_resource_name(project, sa_email));
    let res = http
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    if status.is_success() || status.as_u16() == 404 {
        return Ok(());
    }
    let err = res.text().await.unwrap_or_default();
    Err(format!(
        "IAM delete SA HTTP {status}: {}",
        truncate(&err, 200)
    ))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}
