//! Единый HTTP + rate limit для marketplace-скриптов (шаблоны и калькулятор).
//! Каждый запрос пишется в AppLog с ярлыком скрипта (`[Имя]` / `[Калькулятор][Имя]`).

use super::client::HttpClient;
use super::rate_limit::KeyedRateLimiter;
use super::request_sender_method::{auth_parts, url_path_for_log, Method, OwnedMethod};
use crate::core::log::AppLog;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub use super::request_sender_method::{Auth, Limit};

#[derive(Clone)]
pub struct RequestSender {
    http: HttpClient,
    limiter: KeyedRateLimiter,
    script_key: String,
    /// Ярлык как у ScriptLog: `[Воронка]` или `[Калькулятор][Воронка]`.
    log_label: String,
    log: AppLog,
    cancelled: Arc<AtomicBool>,
}

impl RequestSender {
    pub fn new_with_cancel(
        http: HttpClient,
        limiter: KeyedRateLimiter,
        script_key: impl Into<String>,
        log_label: impl Into<String>,
        log: AppLog,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            http,
            limiter,
            script_key: script_key.into(),
            log_label: log_label.into(),
            log,
            cancelled,
        }
    }

    fn check_cancel(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Отменено".into())
        } else {
            Ok(())
        }
    }

    fn line(&self, message: impl Into<String>) -> String {
        let msg = message.into();
        if self.log_label.is_empty() {
            msg
        } else {
            format!("{}: {msg}", self.log_label)
        }
    }

    pub async fn wait(&self, limit: Limit<'_>) -> Result<Duration, String> {
        self.check_cancel()?;
        let waited = self
            .limiter
            .wait(
                &self.script_key,
                limit.cabinet,
                limit.scope,
                limit.profile,
            )
            .await;
        self.check_cancel()?;
        if waited >= Duration::from_millis(500) {
            self.log.info(self.line(format!(
                "Кабинет {}: ожидание лимита {:.0}с",
                limit.cabinet,
                waited.as_secs_f64()
            )));
        }
        Ok(waited)
    }

    pub async fn get_json(
        &self,
        limit: Limit<'_>,
        url: &str,
        auth: Auth<'_>,
    ) -> Result<Value, String> {
        self.send_json(limit, Method::Get { url, auth }).await
    }

    pub async fn post_json(
        &self,
        limit: Limit<'_>,
        url: &str,
        auth: Auth<'_>,
        body: &Value,
    ) -> Result<Value, String> {
        self.send_json(
            limit,
            Method::Post {
                url,
                auth,
                body: Some(body),
            },
        )
        .await
    }

    pub async fn ozon_get(
        &self,
        limit: Limit<'_>,
        path: &str,
        client_id: &str,
        api_key: &str,
    ) -> Result<Value, String> {
        let desc = format!("Кабинет {}: GET {path}", limit.cabinet);
        self.with_429_retry(limit, &desc, || {
            let http = self.http.clone();
            let path = path.to_string();
            let client_id = client_id.to_string();
            let api_key = api_key.to_string();
            async move { http.ozon_get_json(&path, &client_id, &api_key).await }
        })
        .await
    }

    pub async fn ozon_post(
        &self,
        limit: Limit<'_>,
        path: &str,
        client_id: &str,
        api_key: &str,
        body: &Value,
    ) -> Result<Value, String> {
        let desc = format!("Кабинет {}: POST {path}", limit.cabinet);
        self.with_429_retry(limit, &desc, || {
            let http = self.http.clone();
            let path = path.to_string();
            let client_id = client_id.to_string();
            let api_key = api_key.to_string();
            let body = body.clone();
            async move {
                http.ozon_post_json(&path, &client_id, &api_key, &body)
                    .await
            }
        })
        .await
    }

    pub async fn download_bytes(
        &self,
        limit: Limit<'_>,
        url: &str,
    ) -> Result<Vec<u8>, String> {
        let path = url_path_for_log(url);
        let desc = format!("Кабинет {}: GET {path}", limit.cabinet);
        self.with_429_retry(limit, &desc, || {
            let http = self.http.clone();
            let url = url.to_string();
            async move { http.download_bytes(&url).await }
        })
        .await
    }

    /// Скачать байты с Auth (WB nm-report ZIP/CSV).
    pub async fn download_bytes_auth(
        &self,
        limit: Limit<'_>,
        url: &str,
        auth: Auth<'_>,
    ) -> Result<Vec<u8>, String> {
        let path = url_path_for_log(url);
        let desc = format!("Кабинет {}: GET {path}", limit.cabinet);
        let (bearer, raw) = auth_parts(auth);
        self.with_429_retry(limit, &desc, || {
            let http = self.http.clone();
            let url = url.to_string();
            let bearer = bearer.clone();
            async move {
                http.download_bytes_auth(&url, bearer.as_deref(), !raw)
                    .await
            }
        })
        .await
    }

    async fn send_json(
        &self,
        limit: Limit<'_>,
        method: Method<'_>,
    ) -> Result<Value, String> {
        let desc = match &method {
            Method::Get { url, .. } => {
                format!("Кабинет {}: GET {}", limit.cabinet, url_path_for_log(url))
            }
            Method::Post { url, .. } => {
                format!("Кабинет {}: POST {}", limit.cabinet, url_path_for_log(url))
            }
        };
        self.with_429_retry(limit, &desc, || {
            let http = self.http.clone();
            let method = method.clone_owned();
            async move {
                match method {
                    OwnedMethod::Get { url, bearer, raw } => {
                        if raw {
                            http.get_json_auth(&url, bearer.as_deref(), false).await
                        } else {
                            http.get_json_auth(&url, bearer.as_deref(), true).await
                        }
                    }
                    OwnedMethod::Post {
                        url,
                        bearer,
                        raw,
                        body,
                    } => {
                        if raw {
                            http.post_json_auth(&url, bearer.as_deref(), &body, false)
                                .await
                        } else {
                            http.post_json_auth(&url, bearer.as_deref(), &body, true)
                                .await
                        }
                    }
                }
            }
        })
        .await
    }

    async fn with_429_retry<T, F, Fut>(
        &self,
        limit: Limit<'_>,
        request_desc: &str,
        mut make: F,
    ) -> Result<T, String>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        self.wait(limit).await?;
        self.log.info(self.line(request_desc.to_string()));
        match make().await {
            Ok(v) => Ok(v),
            Err(e) if e.contains("HTTP 429") => {
                self.check_cancel()?;
                let ms = limit.profile.pause_on_429.as_millis();
                self.log.warn(self.line(format!(
                    "Кабинет {}: HTTP 429, ожидание {ms}мс и повтор (попытка 1/2)",
                    limit.cabinet
                )));
                sleep(limit.profile.pause_on_429).await;
                self.wait(limit).await?;
                self.log.info(self.line(format!("повтор: {request_desc}")));
                make().await
            }
            Err(e) => Err(e),
        }
    }
}
