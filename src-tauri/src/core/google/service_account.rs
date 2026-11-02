//! Access token для Google Service Account (JWT bearer). Только для auth/workspace Sheets.

use crate::core::i18n::t;
use crate::core::service_account_cfg;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SCOPES: &str =
    "https://www.googleapis.com/auth/spreadsheets https://www.googleapis.com/auth/drive";

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

static TOKEN_CACHE: Mutex<Option<CachedToken>> = Mutex::new(None);

#[derive(Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

pub fn clear_token_cache() {
    if let Ok(mut g) = TOKEN_CACHE.lock() {
        *g = None;
    }
}

pub async fn user_access_token() -> Result<String, String> {
    if let Ok(g) = TOKEN_CACHE.lock() {
        if let Some(c) = g.as_ref() {
            if Instant::now() + Duration::from_secs(60) < c.expires_at {
                return Ok(c.access_token.clone());
            }
        }
    }
    let raw = service_account_cfg::load_raw()
        .ok_or_else(|| "Service Account не настроен".to_string())?;
    fetch_with_retry(&raw).await
}

/// После create/replace ключа Google token endpoint иногда ещё не знает public key
/// (`invalid_grant` / Invalid JWT Signature) — ждём, пока JWT начнёт приниматься.
pub async fn wait_until_usable(raw: &str) -> Result<(), String> {
    clear_token_cache();
    let _ = fetch_with_retry(raw).await?;
    Ok(())
}

async fn fetch_with_retry(raw: &str) -> Result<String, String> {
    let mut last = String::new();
    for attempt in 0..10u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
        }
        match fetch_and_cache(raw).await {
            Ok(tok) => return Ok(tok),
            Err(e) if is_key_propagation_error(&e) => {
                last = e;
            }
            Err(e) => return Err(e),
        }
    }
    Err(if last.is_empty() {
        t("errors.auth.saTokenNotReady")
    } else {
        last
    })
}

fn is_key_propagation_error(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("invalid_grant")
        || e.contains("invalid jwt signature")
        || e.contains("invalid_jwt")
}

async fn fetch_and_cache(raw: &str) -> Result<String, String> {
    let (client_email, private_key, token_uri) = service_account_cfg::parse_key(raw)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let claims = Claims {
        iss: client_email,
        scope: SCOPES.into(),
        aud: token_uri.clone(),
        iat: now,
        exp: now + 3600,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".into());
    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
        .map_err(|e| format!("Service Account private_key: {e}"))?;
    let assertion =
        encode(&header, &claims, &key).map_err(|e| format!("Service Account JWT: {e}"))?;

    let http = super::http::client();
    let form = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", assertion.as_str()),
    ];
    let res = http
        .post(&token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        let short: String = text.chars().take(240).collect();
        return Err(format!("Service Account token HTTP {status}: {short}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let access_token = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Service Account: нет access_token".to_string())?
        .to_string();
    let expires_in = v
        .get("expires_in")
        .and_then(|x| x.as_u64())
        .unwrap_or(3600)
        .max(120);
    if let Ok(mut g) = TOKEN_CACHE.lock() {
        *g = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        });
    }
    Ok(access_token)
}
