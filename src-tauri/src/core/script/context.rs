use super::ScriptField;
use crate::core::config::AppConfig;
use crate::core::http::{HttpClient, KeyedRateLimiter, RequestSender};
use crate::core::log::AppLog;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type ScriptResult = Result<ScriptOutput, ScriptError>;

#[derive(Debug, Clone, Serialize)]
pub struct ScriptOutput {
    pub rows_written: usize,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("{0}")]
    Message(String),
}

impl From<String> for ScriptError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for ScriptError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

#[derive(Clone)]
pub struct ScriptContext {
    pub log: ScriptLog,
    pub request: RequestSender,
    pub config: AppConfig,
    pub params: HashMap<String, String>,
    /// Кооперативная отмена прогона (шаблон / группа).
    pub cancelled: Arc<AtomicBool>,
}

impl ScriptContext {
    /// `log_label` — уже собранный ярлык (`[Имя]` или `[Калькулятор][Имя]`), см. `format_label`.
    pub fn build_with_cancel(
        log: AppLog,
        http: HttpClient,
        limiter: KeyedRateLimiter,
        config: AppConfig,
        params: HashMap<String, String>,
        script_key: impl Into<String>,
        log_label: impl Into<String>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        let script_key = script_key.into();
        let log_label = log_label.into();
        let script_log = ScriptLog::new(log.clone(), log_label.clone());
        let request =
            RequestSender::new_with_cancel(http, limiter, script_key, log_label, log, cancelled.clone());
        Self {
            log: script_log,
            request,
            config,
            params,
            cancelled,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn check_cancelled(&self) -> Result<(), ScriptError> {
        if self.is_cancelled() {
            Err(ScriptError::Message(crate::core::i18n::t("errors.cancelled")))
        } else {
            Ok(())
        }
    }
}

pub trait Script: Send + Sync {
    fn key(&self) -> &'static str;
    fn title(&self) -> String;
    /// Поля по умолчанию для редактора шаблонов (DefaultFields).
    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
    }
    /// Можно запускать не-админу без своего Google OAuth (по умолчанию нет).
    /// Экспортные скрипты без мутаций маркетплейса оборачивают в `MemberOk` при регистрации.
    fn member_ok(&self) -> bool {
        false
    }
    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>>;
}

/// Обёртка: скрипт доступен участнику без HasOauth (Excel-only через шаблоны).
pub struct MemberOk<S: Script>(pub S);

impl<S: Script> Script for MemberOk<S> {
    fn key(&self) -> &'static str {
        self.0.key()
    }
    fn title(&self) -> String {
        self.0.title()
    }
    fn default_fields(&self) -> Vec<ScriptField> {
        self.0.default_fields()
    }
    fn member_ok(&self) -> bool {
        true
    }
    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        self.0.execute(ctx)
    }
}

pub struct ScriptRegistry {
    scripts: HashMap<&'static str, Arc<dyn Script>>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    pub fn register(&mut self, script: Arc<dyn Script>) {
        self.scripts.insert(script.key(), script);
    }

    pub fn get(&self, key: &str) -> Option<Arc<dyn Script>> {
        self.scripts.get(key).cloned()
    }

    pub fn list(&self) -> Vec<(String, String)> {
        let mut items: Vec<_> = self
            .scripts
            .values()
            .map(|s| (s.key().to_string(), s.title().to_string()))
            .collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    }

    pub fn list_with_meta(&self) -> Vec<(String, String, bool)> {
        let mut items: Vec<_> = self
            .scripts
            .values()
            .map(|s| (s.key().to_string(), s.title().to_string(), s.member_ok()))
            .collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    }

    pub fn is_member_ok(&self, key: &str) -> bool {
        self.get(key).map(|s| s.member_ok()).unwrap_or(false)
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// --- log (merged) ---
/// Ярлык для строк журнала: `[Воронка]` или `[Калькулятор][Воронка]`.
pub fn format_label(script_name: &str, from_calculator: bool) -> String {
    let name = script_name.trim();
    let calc = crate::core::i18n::t("templates.calculatorPrefix");
    if name.is_empty() {
        if from_calculator {
            calc
        } else {
            String::new()
        }
    } else if from_calculator {
        format!("{calc}[{name}]")
    } else {
        format!("[{name}]")
    }
}

#[derive(Clone)]
pub struct ScriptLog {
    inner: AppLog,
    /// Уже собранный ярлык (`[Имя]` или `[Калькулятор][Имя]`).
    label: String,
}

impl ScriptLog {
    pub fn new(inner: AppLog, label: impl Into<String>) -> Self {
        Self {
            inner,
            label: label.into(),
        }
    }

    fn format(&self, message: String) -> String {
        if self.label.is_empty() {
            message
        } else {
            format!("{}: {message}", self.label)
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.inner.info(self.format(message.into()));
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.inner.warn(self.format(message.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_normal_and_calculator() {
        assert_eq!(format_label("Воронка", false), "[Воронка]");
        assert_eq!(
            format_label("Воронка", true),
            "[Калькулятор][Воронка]"
        );
        assert_eq!(format_label("", true), "[Калькулятор]");
    }
}
