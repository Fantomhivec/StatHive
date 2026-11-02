use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use std::time::Duration;

const OZON_BASE: &str = "https://api-seller.ozon.ru";

/// Низкоуровневый HTTP без marketplace rate-limit (лимиты — в `RequestSender`).
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            // StocksScript HttpClient = 90с; analytics-ответы могут быть крупными.
            .timeout(Duration::from_secs(90))
            .user_agent("StatHive/0.1")
            .build()
            .expect("reqwest client");
        Self { client }
    }

    /// `bearer_prefix`: false — common-api (Authorization: raw token).
    pub async fn get_json_auth(
        &self,
        url: &str,
        token: Option<&str>,
        bearer_prefix: bool,
    ) -> Result<serde_json::Value, String> {
        let mut req = self.client.get(url);
        if let Some(raw) = token {
            let mut t = raw.trim();
            if let Some(rest) = t.strip_prefix("Bearer ").or_else(|| t.strip_prefix("bearer ")) {
                t = rest.trim();
            }
            let value = if bearer_prefix {
                format!("Bearer {t}")
            } else {
                t.to_string()
            };
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value).map_err(|e| e.to_string())?,
            );
            req = req.headers(headers);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "HTTP {status}: {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// `bearer_prefix`: false — raw Authorization (advert / analytics API ).
    pub async fn post_json_auth(
        &self,
        url: &str,
        token: Option<&str>,
        body: &serde_json::Value,
        bearer_prefix: bool,
    ) -> Result<serde_json::Value, String> {
        let mut req = self.client.post(url).json(body);
        if let Some(raw) = token {
            let mut t = raw.trim();
            if let Some(rest) = t.strip_prefix("Bearer ").or_else(|| t.strip_prefix("bearer ")) {
                t = rest.trim();
            }
            let value = if bearer_prefix {
                format!("Bearer {t}")
            } else {
                t.to_string()
            };
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value).map_err(|e| e.to_string())?,
            );
            req = req.headers(headers);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "HTTP {status}: {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        if bytes.iter().all(|b| b.is_ascii_whitespace()) {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    fn ozon_url(path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else if path.starts_with('/') {
            format!("{OZON_BASE}{path}")
        } else {
            format!("{OZON_BASE}/{path}")
        }
    }

    fn ozon_auth_headers(client_id: &str, api_key: &str) -> Result<HeaderMap, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("client-id"),
            HeaderValue::from_str(client_id.trim()).map_err(|e| e.to_string())?,
        );
        headers.insert(
            HeaderName::from_static("api-key"),
            HeaderValue::from_str(api_key.trim()).map_err(|e| e.to_string())?,
        );
        Ok(headers)
    }

    async fn ozon_parse_json(res: reqwest::Response) -> Result<serde_json::Value, String> {
        let status = res.status();
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "HTTP {status}: {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        if bytes.iter().all(|b| b.is_ascii_whitespace()) {
            return Ok(serde_json::json!({}));
        }
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    /// Ozon Seller API GET: `Client-Id` + `Api-Key` (например `/v1/actions`).
    pub async fn ozon_get_json(
        &self,
        path: &str,
        client_id: &str,
        api_key: &str,
    ) -> Result<serde_json::Value, String> {
        let url = Self::ozon_url(path);
        let headers = Self::ozon_auth_headers(client_id, api_key)?;
        let res = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::ozon_parse_json(res).await
    }

    /// Ozon Seller API POST: `Client-Id` + `Api-Key`, path вида `/v3/product/list`.
    pub async fn ozon_post_json(
        &self,
        path: &str,
        client_id: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let url = Self::ozon_url(path);
        let headers = Self::ozon_auth_headers(client_id, api_key)?;
        let res = self
            .client
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::ozon_parse_json(res).await
    }

    /// Скачать байты по URL (отчёты Ozon) без Ozon-заголовков.
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        self.download_bytes_auth(url, None, false).await
    }

    /// Скачать байты с Authorization (WB analytics CSV/ZIP).
    /// `bearer_prefix`: false — raw token (как get_json_auth).
    pub async fn download_bytes_auth(
        &self,
        url: &str,
        token: Option<&str>,
        bearer_prefix: bool,
    ) -> Result<Vec<u8>, String> {
        let mut req = self.client.get(url);
        if let Some(raw) = token {
            let mut t = raw.trim();
            if let Some(rest) = t.strip_prefix("Bearer ").or_else(|| t.strip_prefix("bearer ")) {
                t = rest.trim();
            }
            let value = if bearer_prefix {
                format!("Bearer {t}")
            } else {
                t.to_string()
            };
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&value).map_err(|e| e.to_string())?,
            );
            req = req.headers(headers);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!(
                "HTTP {status}: {}",
                String::from_utf8_lossy(&bytes)
            ));
        }
        Ok(bytes.into())
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
