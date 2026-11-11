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

//! Ручные прогоны шаблонов: отмена + событие `templates.runProgress`.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// Replaceable handle: command entry refreshes before a long group run.
static APP: Mutex<Option<AppHandle>> = Mutex::new(None);
/// Lesson 120: emit off the command task + yield so WebView paints between nested runs
/// (same pattern as `calculator::run_log`).
static EMIT_TX: OnceLock<mpsc::UnboundedSender<Value>> = OnceLock::new();
static MANUAL_ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MANUAL_LIMIT: usize = 4;

type CancelMap = Mutex<HashMap<String, Arc<AtomicBool>>>;
/// group_id → current template id while group run is active.
type ActiveTplMap = Mutex<HashMap<String, String>>;

fn cancel_map() -> &'static CancelMap {
    static MAP: OnceLock<CancelMap> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_tpl_map() -> &'static ActiveTplMap {
    static MAP: OnceLock<ActiveTplMap> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn set_app_handle(app: AppHandle) {
    if let Ok(mut slot) = APP.lock() {
        *slot = Some(app.clone());
    }
    if EMIT_TX.get().is_some() {
        return;
    }
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    if EMIT_TX.set(tx).is_err() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        while let Some(payload) = rx.recv().await {
            // Lesson 123: do not silently drop when AppHandle is briefly unset.
            let mut emitted = false;
            for _ in 0..20 {
                let handle = APP.lock().ok().and_then(|g| g.clone());
                if let Some(handle) = handle {
                    let _ = handle.emit("templates.runProgress", &payload);
                    emitted = true;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            if !emitted {
                let handle = APP.lock().ok().and_then(|g| g.clone());
                if let Some(handle) = handle {
                    let _ = handle.emit("templates.runProgress", &payload);
                }
            }
            tokio::task::yield_now().await;
        }
    });
}

pub fn register(run_key: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = cancel_map().lock() {
        g.insert(run_key.to_string(), flag.clone());
    }
    MANUAL_ACTIVE.fetch_add(1, Ordering::Relaxed);
    flag
}

/// Remove key from cancel_map; idempotent (safe if already cleared by cancel/Drop).
pub fn unregister(run_key: &str) {
    let removed = if let Ok(mut g) = cancel_map().lock() {
        g.remove(run_key).is_some()
    } else {
        false
    };
    if !removed {
        return;
    }
    if let Some(gid) = run_key.strip_prefix("group:") {
        clear_group_active_template(gid);
    }
    MANUAL_ACTIVE.fetch_sub(1, Ordering::Relaxed);
}

fn parse_run_key(run_key: &str) -> Option<(&str, &str)> {
    if let Some(id) = run_key.strip_prefix("group:") {
        Some(("group", id))
    } else if let Some(id) = run_key.strip_prefix("template:") {
        Some(("template", id))
    } else {
        None
    }
}

/// Stop: set cancel flag, drop map key (hydrate idle), emit cancelled.
/// Live loops keep their `Arc<AtomicBool>` and still observe the flag.
pub fn cancel(run_key: &str) -> bool {
    let found = if let Ok(g) = cancel_map().lock() {
        if let Some(f) = g.get(run_key) {
            f.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    } else {
        false
    };
    unregister(run_key);
    if let Some((entity_type, id)) = parse_run_key(run_key) {
        let at = chrono::Utc::now().to_rfc3339();
        emit_progress_ex(
            entity_type,
            id,
            Some("cancelled"),
            false,
            Some(&at),
            None,
            if entity_type == "group" {
                Some(id)
            } else {
                None
            },
        );
        if entity_type == "group" {
            let _ = crate::templates::store::set_group_last_run(id, "cancelled");
        } else {
            let _ = crate::templates::store::set_last_run(id, "cancelled");
        }
    }
    found
}

/// Best-effort: cancel every active run (window close).
pub fn cancel_all() {
    for key in active_keys() {
        let _ = cancel(&key);
    }
}

/// RAII: always unregister; on abort (not disarmed) emit cancelled + persist.
pub struct RunGuard {
    run_key: String,
    entity_type: &'static str,
    entity_id: String,
    disarmed: bool,
}

impl RunGuard {
    pub fn arm(run_key: String, entity_type: &'static str, entity_id: String) -> (Arc<AtomicBool>, Self) {
        let flag = register(&run_key);
        (
            flag,
            Self {
                run_key,
                entity_type,
                entity_id,
                disarmed: false,
            },
        )
    }

    /// Normal path finished emit/persist — Drop only unregisters.
    pub fn disarm(mut self) {
        self.disarmed = true;
        unregister(&self.run_key);
        std::mem::forget(self);
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        unregister(&self.run_key);
        if self.disarmed {
            return;
        }
        let at = chrono::Utc::now().to_rfc3339();
        let gid = if self.entity_type == "group" {
            Some(self.entity_id.as_str())
        } else {
            None
        };
        emit_progress_ex(
            self.entity_type,
            &self.entity_id,
            Some("cancelled"),
            false,
            Some(&at),
            None,
            gid,
        );
        if self.entity_type == "group" {
            let _ = crate::templates::store::set_group_last_run(&self.entity_id, "cancelled");
        } else {
            let _ = crate::templates::store::set_last_run(&self.entity_id, "cancelled");
        }
    }
}

/// Snapshot of active run keys (`template:…` / `group:…`).
pub fn active_keys() -> Vec<String> {
    cancel_map()
        .lock()
        .ok()
        .map(|g| g.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn set_group_active_template(group_id: &str, template_id: &str) {
    if let Ok(mut g) = active_tpl_map().lock() {
        g.insert(group_id.to_string(), template_id.to_string());
    }
}

pub fn clear_group_active_template(group_id: &str) {
    if let Ok(mut g) = active_tpl_map().lock() {
        g.remove(group_id);
    }
}

pub fn group_active_template(group_id: &str) -> Option<String> {
    active_tpl_map()
        .lock()
        .ok()
        .and_then(|g| g.get(group_id).cloned())
}

pub fn emit_progress(
    entity_type: &str,
    id: &str,
    status: Option<&str>,
    is_running: bool,
    last_run_at: Option<&str>,
) {
    emit_progress_ex(entity_type, id, status, is_running, last_run_at, None, None);
}

pub fn emit_progress_ex(
    entity_type: &str,
    id: &str,
    status: Option<&str>,
    is_running: bool,
    last_run_at: Option<&str>,
    active_template_id: Option<&str>,
    group_id: Option<&str>,
) {
    let mut payload = json!({
        "entityType": entity_type,
        "id": id,
        "lastRunStatus": status,
        "isRunning": is_running,
        "lastRunAt": last_run_at,
        "manualRunsActive": MANUAL_ACTIVE.load(Ordering::Relaxed),
        "manualRunsLimit": MANUAL_LIMIT,
    });
    if let Some(obj) = payload.as_object_mut() {
        if let Some(tid) = active_template_id {
            obj.insert("activeTemplateId".into(), json!(tid));
        }
        if let Some(gid) = group_id {
            obj.insert("groupId".into(), json!(gid));
        }
    }
    if let Some(tx) = EMIT_TX.get() {
        let _ = tx.send(payload);
        return;
    }
    // Fallback before emitter task is up (should be rare after setup).
    let app = APP.lock().ok().and_then(|g| g.clone());
    if let Some(app) = app {
        let _ = app.emit("templates.runProgress", payload);
    }
}

/// Patch snapshot cards with live run flags from cancel_map / active template.
/// Full reconcile: clear all ephemeral flags first, then set from active_keys only.
pub fn hydrate_snapshot_running(snap: &mut crate::templates::model::TemplatesSnapshot) {
    for g in &mut snap.groups {
        g.is_running = false;
        g.active_template_id = None;
    }
    for t in &mut snap.templates {
        t.is_running = false;
    }
    for t in &mut snap.standalone {
        t.is_running = false;
    }

    let keys = active_keys();
    let mut tpl_running: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut group_running: std::collections::HashSet<String> = std::collections::HashSet::new();
    for k in &keys {
        if let Some(id) = k.strip_prefix("template:") {
            tpl_running.insert(id.to_string());
        } else if let Some(id) = k.strip_prefix("group:") {
            group_running.insert(id.to_string());
        }
    }

    let mut group_member_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for g in &mut snap.groups {
        if group_running.contains(&g.id) {
            g.is_running = true;
            g.active_template_id = group_active_template(&g.id);
            if let Some(ref tid) = g.active_template_id {
                tpl_running.insert(tid.clone());
            }
            for tid in &g.template_ids {
                group_member_ids.insert(tid.clone());
            }
        }
    }

    for t in &mut snap.templates {
        if tpl_running.contains(&t.id) {
            t.is_running = true;
        } else if group_member_ids.contains(&t.id) {
            // Only the active nested is running — clear siblings from prior optimistic UI.
            t.is_running = false;
        }
    }
    for t in &mut snap.standalone {
        if tpl_running.contains(&t.id) {
            t.is_running = true;
        } else if group_member_ids.contains(&t.id) {
            t.is_running = false;
        }
    }
}
