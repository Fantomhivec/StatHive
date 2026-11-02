//! Журнал создания/обновления таблиц калькулятора (сессия + событие UI).

use crate::core::config::AppConfig;
use crate::core::log::{self as app_log_mod, AppLog};
use chrono::Local;
use serde_json::json;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

const MAX_LINES: usize = 4000;

static LINES: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();
static EMIT_TX: OnceLock<mpsc::UnboundedSender<EmitMsg>> = OnceLock::new();
static MIRROR_ON: AtomicBool = AtomicBool::new(false);

enum EmitMsg {
    Line(String),
    Cleared,
}

fn buf() -> &'static Mutex<VecDeque<String>> {
    LINES.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn log_path() -> PathBuf {
    AppConfig::config_dir().join("wb_calculator_runs.log")
}

pub fn set_app_handle(app: AppHandle) {
    let _ = APP.set(app.clone());
    let (tx, mut rx) = mpsc::unbounded_channel::<EmitMsg>();
    let _ = EMIT_TX.set(tx);
    // Отдельная задача: emit + yield, чтобы WebView успевал рисовать между строками
    // (аналог BeginInvoke), а не пачкой после длинного await.
    tauri::async_runtime::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match msg {
                EmitMsg::Line(line) => {
                    let _ = app.emit("wbCalculator.runLog", json!({ "line": line }));
                }
                EmitMsg::Cleared => {
                    let _ = app.emit("wbCalculator.runLogCleared", json!({}));
                }
            }
            tokio::task::yield_now().await;
        }
    });
}

fn enqueue_emit(msg: EmitMsg) {
    if let Some(tx) = EMIT_TX.get() {
        let _ = tx.send(msg);
        return;
    }
    if let Some(app) = APP.get() {
        match msg {
            EmitMsg::Line(line) => {
                let _ = app.emit("wbCalculator.runLog", json!({ "line": line }));
            }
            EmitMsg::Cleared => {
                let _ = app.emit("wbCalculator.runLogCleared", json!({}));
            }
        }
    }
}

/// Пока жив guard: `AppLog::info` дублируется в журнал калькулятора (onLog).
pub struct AppLogMirrorGuard;

impl Drop for AppLogMirrorGuard {
    fn drop(&mut self) {
        MIRROR_ON.store(false, Ordering::SeqCst);
        app_log_mod::set_info_sink(None);
    }
}

pub fn enable_app_log_mirror() -> AppLogMirrorGuard {
    MIRROR_ON.store(true, Ordering::SeqCst);
    app_log_mod::set_info_sink(Some(Arc::new(|msg: &str| {
        append(msg);
    })));
    AppLogMirrorGuard
}

fn mirror_active() -> bool {
    MIRROR_ON.load(Ordering::SeqCst)
}

/// AppLog + журнал калькулятора + emit в UI.
pub fn calc_info(app_log: &AppLog, message: impl Into<String>) {
    let msg = message.into();
    if mirror_active() {
        // sink уже вызовет append из AppLog::info
        app_log.info(msg);
    } else {
        app_log.info_no_sink(msg.clone());
        append(&msg);
    }
}

pub fn append(message: &str) {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return;
    }
    // Миллисекунды — чтобы видеть порядок этапов внутри одной секунды.
    let now = Local::now();
    let ts = format!(
        "{}.{:03}",
        now.format("%d.%m.%Y %H:%M:%S"),
        now.timestamp_subsec_millis()
    );
    let line = format!("{ts} {trimmed}");

    if let Ok(mut q) = buf().lock() {
        q.push_back(line.clone());
        while q.len() > MAX_LINES {
            q.pop_front();
        }
    }

    let _ = (|| -> Result<(), String> {
        let path = log_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{line}").map_err(|e| e.to_string())?;
        Ok(())
    })();

    enqueue_emit(EmitMsg::Line(line));
}

pub fn text() -> String {
    buf()
        .lock()
        .map(|q| q.iter().cloned().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

pub fn lines() -> Vec<String> {
    buf()
        .lock()
        .map(|q| q.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn clear() {
    if let Ok(mut q) = buf().lock() {
        q.clear();
    }
    let path = log_path();
    let _ = std::fs::write(&path, "");
    enqueue_emit(EmitMsg::Cleared);
}
