//! Activity timer + access kick.
use super::super::{AuthService, UserInfo};
use crate::core::access::{self, reload_presets_cache};
use crate::core::i18n::{t, tf};
use chrono::Utc;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use super::super::sheets_users::{
    auth_client, clear_session_cells, read_user_row, spreadsheet_id, write_activity,
};
use super::session::{
    preset_caps_fingerprint, session_caps_changed, sheet_access_revoked,
    sheet_row_matches_session, user_from_sheet,
};

impl AuthService {
    pub fn ensure_activity_timer(&self) {
        let should_start = {
            let Ok(mut g) = self.activity_stop.lock() else {
                return;
            };
            match g.as_ref() {
                None => true,
                Some(flag) if flag.load(Ordering::SeqCst) => {
                    // Kick/stop оставил мёртвый flag в слоте — иначе таймер не поднимется.
                    *g = None;
                    true
                }
                Some(_) => false,
            }
        };
        if should_start && self.is_authorized() {
            let row = self.current_user().map(|u| u.row_index).unwrap_or(0);
            if row > 0 {
                self.start_activity_timer();
            }
        }
    }
    pub(crate) fn start_activity_timer(&self) {
        self.stop_activity_timer();
        let stop = Arc::new(AtomicBool::new(false));
        if let Ok(mut g) = self.activity_stop.lock() {
            *g = Some(stop.clone());
        }
        let current = Arc::clone(&self.current);
        let session_gen = Arc::clone(&self.session_gen);
        let activity_stop = Arc::clone(&self.activity_stop);
        let log = self.log.clone();
        std::thread::Builder::new()
            .name("auth-activity".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        log.warn(tf(
                            "log.auth.activityTimerRuntime",
                            &[("error", &e.to_string())],
                        ));
                        return;
                    }
                };
                rt.block_on(async move {
                    // После N подряд ошибок клиента/чтения Users — fail-closed (не держим старые caps).
                    const ROW_READ_FAIL_LIMIT: u32 = 3;
                    let mut row_read_failures = 0u32;
                    // Успешно доставленный emit; не сдвигать до Ok — иначе тик «молчит» навсегда.
                    let mut last_emitted_fp = String::new();
                    loop {
                        if stop.load(Ordering::SeqCst) {
                            break;
                        }
                        let tick_gen = session_gen.load(Ordering::SeqCst);
                        let snapshot = current.lock().ok().and_then(|g| g.clone());
                        if let Some(user) = snapshot {
                            if user.is_authorized && user.row_index > 0 {
                                let sid = match spreadsheet_id() {
                                    Ok(s) => s,
                                    Err(e) => {
                                        row_read_failures =
                                            row_read_failures.saturating_add(1);
                                        log.warn(tf(
                                            "log.auth.userRowReadFailed",
                                            &[
                                                ("login", &user.login),
                                                ("error", &e),
                                                ("n", &row_read_failures.to_string()),
                                            ],
                                        ));
                                        if row_read_failures >= ROW_READ_FAIL_LIMIT {
                                            activity_kick(
                                                &session_gen,
                                                &activity_stop,
                                                &current,
                                                &user.login,
                                                &log,
                                            );
                                            break;
                                        }
                                        tokio::time::sleep(Duration::from_secs(300)).await;
                                        continue;
                                    }
                                };
                                if session_gen.load(Ordering::SeqCst) != tick_gen {
                                    continue;
                                }
                                let client = match auth_client().await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        row_read_failures =
                                            row_read_failures.saturating_add(1);
                                        log.warn(tf(
                                            "log.auth.userRowReadFailed",
                                            &[
                                                ("login", &user.login),
                                                ("error", &e),
                                                ("n", &row_read_failures.to_string()),
                                            ],
                                        ));
                                        if row_read_failures >= ROW_READ_FAIL_LIMIT {
                                            activity_kick(
                                                &session_gen,
                                                &activity_stop,
                                                &current,
                                                &user.login,
                                                &log,
                                            );
                                            break;
                                        }
                                        tokio::time::sleep(Duration::from_secs(300)).await;
                                        continue;
                                    }
                                };
                                if session_gen.load(Ordering::SeqCst) != tick_gen {
                                    continue;
                                }
                                if let Err(e) = reload_presets_cache(&client, &sid).await {
                                    access::clear_presets_cache();
                                    row_read_failures =
                                        row_read_failures.saturating_add(1);
                                    log.warn(tf(
                                        "log.auth.userRowReadFailed",
                                        &[
                                            ("login", &user.login),
                                            ("error", &e),
                                            ("n", &row_read_failures.to_string()),
                                        ],
                                    ));
                                    if row_read_failures >= ROW_READ_FAIL_LIMIT {
                                        activity_kick(
                                            &session_gen,
                                            &activity_stop,
                                            &current,
                                            &user.login,
                                            &log,
                                        );
                                        break;
                                    }
                                    tokio::time::sleep(Duration::from_secs(300)).await;
                                    continue;
                                }
                                if session_gen.load(Ordering::SeqCst) != tick_gen {
                                    continue;
                                }
                                let sheet_level = match read_user_row(
                                    &client,
                                    &sid,
                                    user.row_index,
                                )
                                .await
                                {
                                    Ok(row) => {
                                        if session_gen.load(Ordering::SeqCst) != tick_gen {
                                            continue;
                                        }
                                        row_read_failures = 0;
                                        let revoked = sheet_access_revoked(&row.access_level);
                                        let sid_mismatch = !sheet_row_matches_session(&user, &row);
                                        if revoked || sid_mismatch {
                                            if sid_mismatch && !revoked {
                                                log.warn(tf(
                                                    "log.auth.sessionIdMismatchKick",
                                                    &[("login", &user.login)],
                                                ));
                                            }
                                            if let Err(e) = clear_session_cells(
                                                &client,
                                                &sid,
                                                user.row_index,
                                            )
                                            .await
                                            {
                                                log.warn(tf(
                                                    "log.auth.logoutSheetClearFailed",
                                                    &[("error", &e)],
                                                ));
                                            }
                                            activity_kick(
                                                &session_gen,
                                                &activity_stop,
                                                &current,
                                                &user.login,
                                                &log,
                                            );
                                            break;
                                        }
                                        let session_id = user.session_id.clone().unwrap_or_else(
                                            || row.session_id.clone().unwrap_or_default(),
                                        );
                                        let next = user_from_sheet(&row, session_id);
                                        let next_fp =
                                            preset_caps_fingerprint(&next.access_level);
                                        let caps_changed = session_caps_changed(&user, &next)
                                            || next_fp != last_emitted_fp;
                                        if caps_changed {
                                            if let Ok(raw) = serde_json::to_string_pretty(
                                                &super::super::SessionFile {
                                                    login: next.login.clone(),
                                                    display_name: next.display_name.clone(),
                                                    access_level: next.access_level.clone(),
                                                    row_index: next.row_index,
                                                    session_id: next.session_id.clone(),
                                                    birth_date: next.birth_date.clone(),
                                                    last_login: next.last_login.clone(),
                                                    google_email: next.google_email.clone(),
                                                    has_oauth: next.has_oauth,
                                                },
                                            ) {
                                                let _ = AuthService::write_session_file(&raw);
                                            }
                                            if let Ok(mut g) = current.lock() {
                                                *g = Some(next.clone());
                                            }
                                            if super::super::emit_session_updated(&next) {
                                                last_emitted_fp = next_fp;
                                            } else {
                                                log.warn(tf(
                                                    "log.auth.sessionUpdatedEmitFailed",
                                                    &[("login", &next.login)],
                                                ));
                                            }
                                        }
                                        Some(next.access_level)
                                    }
                                    Err(e) => {
                                        row_read_failures =
                                            row_read_failures.saturating_add(1);
                                        log.warn(tf(
                                            "log.auth.userRowReadFailed",
                                            &[
                                                ("login", &user.login),
                                                ("error", &e),
                                                ("n", &row_read_failures.to_string()),
                                            ],
                                        ));
                                        if row_read_failures >= ROW_READ_FAIL_LIMIT {
                                            activity_kick(
                                                &session_gen,
                                                &activity_stop,
                                                &current,
                                                &user.login,
                                                &log,
                                            );
                                            break;
                                        }
                                        Some(user.access_level.clone())
                                    }
                                };
                                let Some(sheet_level) = sheet_level else {
                                    break;
                                };
                                if let Err(e) =
                                    write_activity(&client, &sid, user.row_index, Utc::now())
                                        .await
                                {
                                    log.warn(tf(
                                        "log.auth.sessionActivityFailed",
                                        &[("error", &e.to_string())],
                                    ));
                                }
                                if let Err(e) = crate::calculator::sync_assigned_config(
                                    &client,
                                    &sid,
                                    user.row_index,
                                    &sheet_level,
                                    &log,
                                )
                                .await
                                {
                                    log.warn(tf(
                                        "log.calc.teamPullFailed",
                                        &[("error", &e)],
                                    ));
                                }
                            }
                        }
                        tokio::time::sleep(Duration::from_secs(300)).await;
                    }
                });
            })
            .ok();
    }

    pub(crate) fn stop_activity_timer(&self) {
        if let Ok(mut g) = self.activity_stop.lock() {
            if let Some(flag) = g.take() {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

fn activity_kick(
    session_gen: &std::sync::atomic::AtomicU64,
    activity_stop: &std::sync::Mutex<Option<Arc<AtomicBool>>>,
    current: &std::sync::Mutex<Option<UserInfo>>,
    login: &str,
    log: &crate::core::log::AppLog,
) {
    let reason = t("errors.auth.noProjectAccess");
    log.warn(tf("log.auth.accessRevokedKick", &[("login", login)]));
    session_gen.fetch_add(1, Ordering::SeqCst);
    // take() — слот пуст, ensure_activity_timer сможет стартовать снова после логина.
    if let Ok(mut g) = activity_stop.lock() {
        if let Some(flag) = g.take() {
            flag.store(true, Ordering::SeqCst);
        }
    }
    let _ = fs::remove_file(AuthService::session_path());
    if let Ok(mut g) = current.lock() {
        *g = None;
    }
    crate::calculator::article_index::clear();
    // Как logout: не оставлять Google token следующему логину на этом ПК.
    crate::auth::team_oauth_sheet::reset_auto_pull_gate();
    crate::core::google_oauth::clear_cached_token();
    super::super::emit_access_revoked(&reason);
}
