//! Shared reqwest client for Google APIs (Sheets / OAuth / SA / IAM).
//! Separate from marketplace `HttpClient` (rate limits).

use std::sync::OnceLock;
use std::time::Duration;

use tokio::time::sleep;

const USER_AGENT: &str = "StatHive/0.1";
const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// 429 / quota: attempts after the first failure (total tries = 1 + MAX_RETRIES).
const MAX_RETRIES: u32 = 4;
const BASE_BACKOFF_MS: u64 = 1_000;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Process-wide Google HTTP client (timeout 120s).
pub fn client() -> reqwest::Client {
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .user_agent(USER_AGENT)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        })
        .clone()
}

/// HTTP 429 or Sheets/Drive quota-style 403.
pub fn is_rate_or_quota(status: u16, body: &str) -> bool {
    if status == 429 {
        return true;
    }
    if status != 403 {
        return false;
    }
    let b = body.to_ascii_lowercase();
    b.contains("ratelimitexceeded")
        || b.contains("userratelimitexceeded")
        || b.contains("quotaexceeded")
        || b.contains("rate limit")
        || b.contains("quota")
}

pub fn error_is_rate_or_quota(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("http 429")
        || e.contains("ratelimitexceeded")
        || e.contains("userratelimitexceeded")
        || e.contains("quotaexceeded")
        || (e.contains("http 403") && e.contains("quota"))
}

fn retry_after_ms(res: &reqwest::Response, attempt: u32) -> u64 {
    if let Some(h) = res.headers().get(reqwest::header::RETRY_AFTER) {
        if let Ok(s) = h.to_str() {
            if let Ok(secs) = s.parse::<u64>() {
                return secs.saturating_mul(1000).min(60_000);
            }
        }
    }
    BASE_BACKOFF_MS
        .saturating_mul(1u64 << attempt.min(4))
        .min(30_000)
}

/// Send Google request; on 429 / quota 403 — backoff (Retry-After or exponential) and rebuild.
pub async fn exchange_with_backoff<F, Fut>(
    mut send: F,
) -> Result<(reqwest::StatusCode, String), String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
{
    let mut attempt = 0u32;
    loop {
        let res = send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        let code = status.as_u16();
        let wait_ms = retry_after_ms(&res, attempt);
        let text = res.text().await.unwrap_or_default();
        if is_rate_or_quota(code, &text) && attempt < MAX_RETRIES {
            sleep(Duration::from_millis(wait_ms)).await;
            attempt += 1;
            continue;
        }
        return Ok((status, text));
    }
}
