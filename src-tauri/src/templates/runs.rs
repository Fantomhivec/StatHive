//! Ручные прогоны шаблонов: отмена + событие `templates.runProgress`.

use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();
static MANUAL_ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MANUAL_LIMIT: usize = 4;

type CancelMap = Mutex<HashMap<String, Arc<AtomicBool>>>;

fn cancel_map() -> &'static CancelMap {
    static MAP: OnceLock<CancelMap> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn set_app_handle(app: AppHandle) {
    let _ = APP.set(app);
}

pub fn register(run_key: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = cancel_map().lock() {
        g.insert(run_key.to_string(), flag.clone());
    }
    MANUAL_ACTIVE.fetch_add(1, Ordering::Relaxed);
    flag
}

pub fn unregister(run_key: &str) {
    if let Ok(mut g) = cancel_map().lock() {
        g.remove(run_key);
    }
    MANUAL_ACTIVE.fetch_sub(1, Ordering::Relaxed);
}

pub fn cancel(run_key: &str) -> bool {
    if let Ok(g) = cancel_map().lock() {
        if let Some(f) = g.get(run_key) {
            f.store(true, Ordering::Relaxed);
            return true;
        }
    }
    false
}

pub fn emit_progress(
    entity_type: &str,
    id: &str,
    status: Option<&str>,
    is_running: bool,
    last_run_at: Option<&str>,
) {
    let Some(app) = APP.get() else {
        return;
    };
    let payload = json!({
        "entityType": entity_type,
        "id": id,
        "lastRunStatus": status,
        "isRunning": is_running,
        "lastRunAt": last_run_at,
        "manualRunsActive": MANUAL_ACTIVE.load(Ordering::Relaxed),
        "manualRunsLimit": MANUAL_LIMIT,
    });
    let _ = app.emit("templates.runProgress", payload);
}
