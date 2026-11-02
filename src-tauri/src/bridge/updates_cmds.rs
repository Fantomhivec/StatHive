use crate::app_state::AppState;
use crate::core::app_version;
use crate::updates::{UpdateChecker, UpdateInfo};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::State;

/// Success cache: avoid burning unauthenticated GitHub API (60 req/h/IP).
const OK_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// After rate-limit / network error: do not retry for a while.
const ERR_TTL: Duration = Duration::from_secs(30 * 60);

struct OkCache {
    at: Instant,
    info: UpdateInfo,
}

static OK_CACHE: Mutex<Option<OkCache>> = Mutex::new(None);
static ERR_AT: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Deserialize)]
pub struct UpdatesCheckArgs {
    /// `true` — кнопка «Проверить обновления» (подробные info-логи).
    /// `false` / omit — авто при открытии профиля / фоновый диалог (только ошибки).
    #[serde(default)]
    pub verbose: bool,
}

fn soft_skip(current: &str, status: impl Into<String>) -> UpdateInfo {
    let current = app_version::normalize(current);
    UpdateInfo {
        version: current.clone(),
        download_url: String::new(),
        body: String::new(),
        is_newer: false,
        current_version: current.clone(),
        latest_version: current,
        has_update: false,
        status: status.into(),
        releases_text: String::new(),
        asset_digest: String::new(),
    }
}

fn cache_get_ok(ttl: Duration) -> Option<UpdateInfo> {
    let g = OK_CACHE.lock().ok()?;
    let c = g.as_ref()?;
    if c.at.elapsed() <= ttl {
        Some(c.info.clone())
    } else {
        None
    }
}

fn cache_set_ok(info: &UpdateInfo) {
    if let Ok(mut g) = OK_CACHE.lock() {
        *g = Some(OkCache {
            at: Instant::now(),
            info: info.clone(),
        });
    }
    if let Ok(mut e) = ERR_AT.lock() {
        *e = None;
    }
}

fn err_cooldown_active() -> bool {
    ERR_AT
        .lock()
        .ok()
        .and_then(|g| g.as_ref().copied())
        .is_some_and(|at| at.elapsed() <= ERR_TTL)
}

fn mark_err() {
    if let Ok(mut e) = ERR_AT.lock() {
        *e = Some(Instant::now());
    }
}

#[tauri::command]
pub async fn updates_check(
    state: State<'_, Arc<AppState>>,
    args: Option<UpdatesCheckArgs>,
) -> Result<UpdateInfo, String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    let verbose = args.map(|a| a.verbose).unwrap_or(false);
    let current = app_version::current();

    // Quiet: reuse last good result (Profile + UpdateAvailableDialog share this).
    if !verbose {
        if let Some(info) = cache_get_ok(OK_TTL) {
            if let Ok(mut g) = state.last_update.lock() {
                *g = Some(info.clone());
            }
            return Ok(info);
        }
        if err_cooldown_active() {
            let info = soft_skip(
                &current,
                "Проверка обновлений отложена (лимит GitHub API). Повторите позже кнопкой.",
            );
            return Ok(info);
        }
    } else if err_cooldown_active() {
        // Manual button still blocked briefly after 403 so we do not deepen the ban.
        if let Some(info) = cache_get_ok(OK_TTL) {
            return Ok(info);
        }
        return Err(
            "GitHub API временно ограничен (rate limit). Подождите ~30 мин и повторите."
                .into(),
        );
    }

    let checker = UpdateChecker::new(state.log.clone(), current);
    match checker.check(verbose).await {
        Ok(info) => {
            cache_set_ok(&info);
            if let Ok(mut g) = state.last_update.lock() {
                *g = Some(info.clone());
            }
            Ok(info)
        }
        Err(e) => {
            mark_err();
            if !verbose {
                // Soft: keep UI quiet; next quiet calls hit cooldown without HTTP.
                Ok(soft_skip(
                    &app_version::current(),
                    "Проверка обновлений временно недоступна (лимит GitHub). Повторите позже.",
                ))
            } else {
                Err(e)
            }
        }
    }
}

/// Скачать zip в UI с прогрессом и перезапустить через скрипт (; без console updater).
#[tauri::command]
pub async fn updates_download_install(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    let info = state
        .last_update
        .lock()
        .map_err(|_| "lock")?
        .clone()
        .ok_or_else(|| "Нет данных обновления — сначала updates_check".to_string())?;
    state.log.info(format!("Загрузка обновления {}…", info.version));
    if info.download_url.is_empty() {
        let msg = "Нет URL загрузки обновления (проверьте zip-asset релиза для этой ОС).";
        state.log.error(msg);
        return Err(msg.into());
    }
    match crate::updates::download_and_schedule_install(&app, &info, &state.log).await {
        Ok(msg) => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(900)).await;
                app2.exit(0);
            });
            Ok(serde_json::json!({ "success": true, "message": msg }))
        }
        Err(e) => {
            state.log.error(format!("Обновление не установлено: {e}"));
            Err(e)
        }
    }
}
