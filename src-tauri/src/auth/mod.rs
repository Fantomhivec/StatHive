//! Авторизация: workspace Sheets + Service Account (лист «Пользователи») + user OAuth (скрипты).

pub(crate) mod sheets_users;
mod sa_ops;
pub(crate) mod team;
pub(crate) mod team_oauth_sheet;
mod login;

use crate::core::access::{self, reload_presets_cache};
use crate::core::i18n::{t, tf};
use crate::core::google_oauth;
use crate::core::log::AppLog;
use crate::core::oauth_config;
use crate::core::service_account_cfg;
use crate::core::workspace_cfg;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sheets_users::{auth_client, spreadsheet_id};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: AppHandle) {
    let _ = APP.set(app);
}

pub(crate) fn emit_access_revoked(reason: &str) {
    if let Some(app) = APP.get() {
        let _ = app.emit("auth.accessRevoked", json!({ "reason": reason }));
    }
}

/// Сессия обновлена с листа (смена пресета / HasOauth) — UI сбрасывает caps + layout.
/// `true` только если оба emit успешны (есть AppHandle + state для layout).
pub(crate) fn emit_session_updated(user: &UserInfo) -> bool {
    let Some(app) = APP.get() else {
        return false;
    };
    let session_ok = app.emit("auth.sessionUpdated", user).is_ok();
    let layout_ok = emit_tabs_layout_changed(app, user);
    session_ok && layout_ok
}

/// Тот же payload, что после OAuth / save keys — Shell и caps без рестарта.
fn emit_tabs_layout_changed(app: &AppHandle, user: &UserInfo) -> bool {
    let Some(state) = app.try_state::<Arc<crate::app_state::AppState>>() else {
        return false;
    };
    let any_ok = crate::templates::access_gate::any_member_ok_in_scope(
        state.inner().as_ref(),
        &user.access_level,
    );
    let oauth = access::effective_has_oauth(&user.access_level, user.has_oauth);
    let can_check =
        crate::core::oauth_config::can_check_google(&user.access_level, &user.login);
    let layout = crate::core::tabs_layout::get_layout(
        &user.login,
        &user.access_level,
        oauth,
        any_ok,
        can_check,
    );
    app.emit("tabsLayout.changed", &layout).is_ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub login: String,
    pub display_name: String,
    pub is_authorized: bool,
    #[serde(default)]
    pub access_level: String,
    #[serde(default)]
    pub row_index: u32,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub birth_date: String,
    #[serde(default)]
    pub last_login: String,
    #[serde(default)]
    pub google_email: String,
    /// Свой Google OAuth подключён (колонка HasOauth / локальная сессия).
    #[serde(default)]
    pub has_oauth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSetupStatus {
    pub oauth_configured: bool,
    /// Укороченный client_id Desktop OAuth (без secret).
    #[serde(default)]
    pub oauth_client_id_hint: Option<String>,
    pub google_connected: bool,
    pub google_email: Option<String>,
    pub workspace_configured: bool,
    pub workspace_id: Option<String>,
    pub workspace_title: String,
    /// Нет локального workspace → можно создать Admin.
    pub can_register_admin: bool,
    /// Service Account для листа «Пользователи» (без Drive writer участникам).
    pub service_account_configured: bool,
    pub service_account_email: Option<String>,
    /// Вход по коду команды: почта сверяется с листом, браузер только для почты Admin.
    #[serde(default)]
    pub joined_by_code: bool,
    /// Сохранённые команды на этом ПК.
    #[serde(default)]
    pub teams: Vec<crate::core::team_profile::TeamIndexEntry>,
    #[serde(default)]
    pub active_team_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchTeamResult {
    /// `ok` | `needsLogin`
    pub status: String,
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamUserRow {
    pub login: String,
    pub display_name: String,
    pub access_level: String,
    pub google_email: String,
    pub row_index: u32,
    #[serde(default)]
    pub calc_config_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsersDashboard {
    pub team: Vec<TeamUserRow>,
    pub presets: Vec<access::PresetDef>,
    pub workspace_id: Option<String>,
    pub workspace_title: String,
    pub service_account_configured: bool,
    pub service_account_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionFile {
    pub login: String,
    pub display_name: String,
    #[serde(default)]
    pub access_level: String,
    #[serde(default)]
    pub row_index: u32,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub birth_date: String,
    #[serde(default)]
    pub last_login: String,
    #[serde(default)]
    pub google_email: String,
    #[serde(default)]
    pub has_oauth: bool,
}

pub struct AuthService {
    pub(crate) log: AppLog,
    pub(crate) current: Arc<Mutex<Option<UserInfo>>>,
    /// Arc — поток activity/kick может сделать `take()` и освободить слот для ensure.
    pub(crate) activity_stop: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    /// Растёт на каждом выходе. Отложенный refresh не должен заново записать сессию.
    pub(crate) session_gen: Arc<AtomicU64>,
}

impl AuthService {
    pub fn new(log: AppLog) -> Self {
        let svc = Self {
            log,
            current: Arc::new(Mutex::new(None)),
            activity_stop: Arc::new(Mutex::new(None)),
            session_gen: Arc::new(AtomicU64::new(0)),
        };
        svc.load_session();
        svc
    }

    pub(crate) fn session_path() -> PathBuf {
        crate::core::team_profile::active_dir().join("session.json")
    }

    /// Plaintext JSON (как workspace/SA): не зависит от keyring. Старый sealed — читаем и мигрируем.
    pub(crate) fn write_session_file(raw_json: &str) -> Result<(), String> {
        let path = Self::session_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::write(&path, raw_json.trim()).map_err(|e| e.to_string())
    }

    pub(crate) fn load_session(&self) {
        let path = Self::session_path();
        let raw = match crate::core::secure_store::read_text_result(&path) {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(e) => {
                self.log.warn(tf(
                    "log.auth.sessionLoadFailed",
                    &[("error", &e)],
                ));
                let _ = fs::remove_file(&path);
                return;
            }
        };
        // Sealed → plaintext, чтобы следующий старт не зависел от keyring.
        if fs::read_to_string(&path)
            .ok()
            .is_some_and(|disk| disk.trim_start().starts_with("STATHIVE_ATREST_V1."))
        {
            let _ = Self::write_session_file(&raw);
        }
        if let Ok(s) = serde_json::from_str::<SessionFile>(&raw) {
            let user = UserInfo {
                login: s.login,
                display_name: s.display_name,
                is_authorized: true,
                access_level: s.access_level,
                row_index: s.row_index,
                session_id: s.session_id,
                birth_date: s.birth_date,
                last_login: s.last_login,
                google_email: s.google_email,
                has_oauth: s.has_oauth,
            };
            if let Ok(mut g) = self.current.lock() {
                *g = Some(user);
            }
        }
    }

    /// Подтянуть пресеты в кэш (нужно после restore session — иначе can_calculator/templates = false).
    pub async fn ensure_presets_loaded(&self) {
        let Some(user) = self.current_user().filter(|u| u.is_authorized) else {
            return;
        };
        if access::is_admin_role(&user.access_level) {
            return;
        }
        if access::find_preset(&user.access_level).is_some() {
            return;
        }
        let Ok(sid) = spreadsheet_id() else {
            return;
        };
        let Ok(client) = auth_client().await else {
            self.log.warn(t("log.auth.presetsReloadNoClient"));
            return;
        };
        match reload_presets_cache(&client, &sid).await {
            Ok(list) => {
                let n = list.len().to_string();
                self.log.info(tf("log.auth.presetsReloaded", &[("count", &n)]));
            }
            Err(e) => {
                self.log.warn(tf(
                    "log.auth.presetsReloadFailed",
                    &[("error", &e.to_string())],
                ));
            }
        }
    }

    pub(crate) fn save_session(&self, user: &UserInfo) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(&SessionFile {
            login: user.login.clone(),
            display_name: user.display_name.clone(),
            access_level: user.access_level.clone(),
            row_index: user.row_index,
            session_id: user.session_id.clone(),
            birth_date: user.birth_date.clone(),
            last_login: user.last_login.clone(),
            google_email: user.google_email.clone(),
            has_oauth: user.has_oauth,
        })
        .map_err(|e| e.to_string())?;
        Self::write_session_file(&raw)
    }

    pub fn current_user(&self) -> Option<UserInfo> {
        self.current.lock().ok().and_then(|g| g.clone())
    }

    pub fn is_authorized(&self) -> bool {
        self.current_user()
            .map(|u| u.is_authorized)
            .unwrap_or(false)
    }

    pub fn setup_status(&self) -> AuthSetupStatus {
        let ws = workspace_cfg::load();
        let ws_id = {
            let t = ws.spreadsheet_id.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        };
        let show_sa_email = self
            .current_user()
            .filter(|u| u.is_authorized && access::can_manage_team(&u.access_level))
            .is_some();
        AuthSetupStatus {
            oauth_configured: oauth_config::is_configured(),
            oauth_client_id_hint: oauth_config::client_id_hint(),
            google_connected: google_oauth::has_cached_token(),
            google_email: google_oauth::cached_email(),
            workspace_configured: ws_id.is_some(),
            workspace_id: ws_id.clone(),
            workspace_title: ws.title,
            can_register_admin: ws_id.is_none(),
            service_account_configured: service_account_cfg::is_configured(),
            service_account_email: if show_sa_email {
                service_account_cfg::client_email()
            } else {
                None
            },
            joined_by_code: ws.joined_by_code,
            teams: crate::core::team_profile::list_teams(),
            active_team_id: crate::core::team_profile::active_id(),
        }
    }

    pub fn save_oauth_client(&self, raw_json: &str) -> Result<(), String> {
        oauth_config::save_raw(raw_json)?;
        // Новый client → старый user-token недействителен; дальше Connect Google с нуля.
        google_oauth::clear_cached_token();
        self.log.info(t("log.auth.oauthSaved"));
        Ok(())
    }
    pub async fn connect_google(&self) -> Result<String, String> {
        if !oauth_config::is_configured() {
            return Err(t("errors.auth.saveOAuthFirst"));
        }
        let _ = google_oauth::authorize_interactive().await?;
        let email = google_oauth::cached_email().unwrap_or_default();
        self.log.info(tf("log.auth.googleConnected", &[("email", &email)]));
        Ok(email)
    }

    /// Сбросить сохранённый Google-токен и выбрать другой аккаунт в браузере.
    /// После кода команды браузер только у Admin; участнику почта сверяется без OAuth.
    pub async fn switch_google_account(&self) -> Result<String, String> {
        if workspace_cfg::load().joined_by_code {
            let admin = self
                .current_user()
                .filter(|u| u.is_authorized && access::is_admin_role(&u.access_level));
            if admin.is_none() {
                return Err(t("errors.auth.memberNoGoogleBrowser"));
            }
        }
        if !oauth_config::is_configured() {
            return Err(t("errors.auth.saveOAuthFirst"));
        }
        let _ = google_oauth::authorize_switch_account().await?;
        let email = google_oauth::cached_email().unwrap_or_default();
        self.log.info(tf("log.auth.googleSwitched", &[("email", &email)]));
        Ok(email)
    }
    pub fn join_workspace(&self, spreadsheet_id: &str, title: &str) -> Result<(), String> {
        let id = spreadsheet_id.trim();
        self.log.info(tf("log.auth.joinWorkspace", &[("id", id)]));
        workspace_cfg::set_spreadsheet(id, title)?;
        self.log.info(t("log.auth.workspaceJoined"));
        Ok(())
    }
}
