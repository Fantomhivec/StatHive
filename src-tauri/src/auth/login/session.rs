//! Login / refresh / password / logout.
use super::super::{AuthService, UserInfo};
use crate::core::access::{self, reload_presets_cache};
use crate::core::i18n::{t, tf};
use crate::core::google_oauth;
use crate::core::oauth_config;
use crate::core::workspace_cfg;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::atomic::Ordering;
use uuid::Uuid;
use super::super::sheets_users::{
    auth_client, clear_session_cells, find_user_by_credentials,
    find_user_by_credentials_scan, has_active_foreign_session, read_user_row, spreadsheet_id,
    write_last_login, write_password, write_session, SheetUser,
};

impl AuthService {
    pub async fn login(
        &self,
        login: &str,
        password: &str,
        google_email: &str,
    ) -> Result<UserInfo, String> {
        let login = login.trim();
        if login.is_empty() || password.is_empty() {
            return Err(t("errors.auth.enterLoginPassword"));
        }
        // OAuth Desktop не обязателен для входа (Users читает SA). Нужен для «Подключить Google».
        if !workspace_cfg::is_configured() {
            return Err(t("errors.auth.createWorkspaceFirst"));
        }
        let sid = spreadsheet_id()?;
        self.log.info(tf(
            "log.auth.loginStart",
            &[("login", login), ("workspaceId", &sid)],
        ));
        let client = auth_client().await?;
        // Один GET A2:J1000: credentials + email Admin (раньше find + list_users).
        let (found, admin_mail, needs_rehash) =
            find_user_by_credentials_scan(&client, &sid, login, password)
                .await
                .map_err(|e| {
                    self.log.warn(tf(
                        "log.auth.loginCredentialsFailed",
                        &[("error", &e.to_string())],
                    ));
                    e
                })?;
        if needs_rehash {
            let _ = write_password(&client, &sid, found.row_index, password).await;
        }
        let row_s = found.row_index.to_string();
        self.log.info(tf(
            "log.auth.loginUserFound",
            &[("row", &row_s), ("role", &found.access_level)],
        ));
        let entered = google_email.trim();
        if entered.is_empty() {
            return Err(t("errors.auth.teamGoogleRequired"));
        }
        let expected = found.google_email.trim();
        if !expected.is_empty() && !entered.eq_ignore_ascii_case(expected) {
            self.log.warn(tf(
                "log.auth.loginGoogleMismatch",
                &[("oauthEmail", entered), ("expected", expected)],
            ));
            return Err(tf(
                "errors.auth.googleEmailMismatch",
                &[("oauthEmail", entered), ("expected", expected)],
            ));
        }
        // Вход сверяет Users через SA. Браузер Google — только Admin на ПК,
        // где команда создана (не joined_by_code). Участнику OAuth не нужен для входа.
        let joined = workspace_cfg::load().joined_by_code;
        if !joined
            && !admin_mail.is_empty()
            && entered.eq_ignore_ascii_case(&admin_mail)
        {
            let cached = google_oauth::cached_email().unwrap_or_default();
            if !google_oauth::has_cached_token() || !cached.eq_ignore_ascii_case(entered) {
                self.log.info(t("log.auth.loginNoToken"));
                google_oauth::clear_cached_token();
                let _ = google_oauth::authorize_interactive().await?;
            }
        } else {
            self.log.info(tf(
                "log.auth.loginMemberEmail",
                &[("email", entered)],
            ));
        }
        self.finish_login(&client, &sid, found).await
    }

    pub(crate) async fn login_with_client(
        &self,
        client: &crate::core::google_sheets::GoogleSheetsClient,
        sid: &str,
        login: &str,
        password: &str,
        _row: u32,
    ) -> Result<UserInfo, String> {
        let found = find_user_by_credentials(client, sid, login, password).await?;
        self.finish_login(client, sid, found).await
    }

    pub(crate) async fn finish_login(
        &self,
        client: &crate::core::google_sheets::GoogleSheetsClient,
        sid: &str,
        found: SheetUser,
    ) -> Result<UserInfo, String> {
        let machine = machine_id();
        if has_active_foreign_session(&found, &machine) {
            self.log.warn(tf(
                "log.auth.loginAlreadyElsewhere",
                &[("login", &found.login)],
            ));
            return Err(t("errors.auth.alreadyLoggedInElsewhere"));
        }
        // Сверка Google email только у Admin (у него свой OAuth). Участник в Testing не входит.
        if access::is_admin_role(&found.access_level) {
            if let Some(oauth_email) = google_oauth::cached_email() {
            let expected = found.google_email.trim();
            if !expected.is_empty()
                && !oauth_email.eq_ignore_ascii_case(expected)
            {
                self.log.warn(tf(
                    "log.auth.loginGoogleMismatch",
                    &[("oauthEmail", &oauth_email), ("expected", expected)],
                ));
                return Err(tf(
                    "errors.auth.googleEmailMismatch",
                    &[("oauthEmail", &oauth_email), ("expected", expected)],
                ));
            }
            if !expected.is_empty() {
                self.log.info(tf(
                    "log.auth.loginGoogleMatch",
                    &[("oauthEmail", &oauth_email)],
                ));
            }
            }
        }
        let _ = reload_presets_cache(client, sid).await;
        if !access::is_admin_role(&found.access_level)
            && access::find_preset(&found.access_level).is_none()
        {
            return Err(t("errors.auth.noProjectAccess"));
        }
        let gen = self.session_gen.load(Ordering::SeqCst);
        let now = Utc::now();
        let session_id = format!("{}_{}", Uuid::new_v4(), machine);
        let user = user_from_sheet(&found, session_id.clone());
        write_session(client, sid, found.row_index, &session_id, now).await?;
        let _ = write_last_login(client, sid, found.row_index, now).await;
        if self.session_gen.load(Ordering::SeqCst) != gen {
            return Err(t("errors.auth.userNotAuthorized"));
        }
        // Назначенная конфигурация: первый sync в activity timer (сразу + каждые 5 мин).
        self.save_session(&user)?;
        let role = if access::is_admin_role(&user.access_level) {
            "Admin"
        } else {
            "Member"
        };
        let _ = crate::core::team_profile::update_last_role(role);
        if let Ok(mut g) = self.current.lock() {
            *g = Some(user.clone());
        }
        self.start_activity_timer();
        self.log.info(t("log.auth.sessionCreated"));
        crate::auth::team_oauth_sheet::auto_pull_if_needed(
            &user.access_level,
            &user.login,
            &self.log,
        )
        .await;
        Ok(user)
    }

    pub async fn refresh_profile(&self) -> Result<UserInfo, String> {
        let gen = self.session_gen.load(Ordering::SeqCst);
        let current = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.userNotAuthorized"))?;
        if current.row_index < 2 {
            return Ok(current);
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let _ = reload_presets_cache(&client, &sid).await;
        let found = read_user_row(&client, &sid, current.row_index).await?;
        if self.session_gen.load(Ordering::SeqCst) != gen
            || self
                .current_user()
                .filter(|u| u.is_authorized)
                .is_none()
        {
            return Err(t("errors.auth.userNotAuthorized"));
        }
        if sheet_access_revoked(&found.access_level)
            || !sheet_row_matches_session(&current, &found)
        {
            self.revoke_live_session(&current, &client, &sid).await;
            return Err(t("errors.auth.noProjectAccess"));
        }
        let session_id = current
            .session_id
            .clone()
            .unwrap_or_else(|| found.session_id.clone().unwrap_or_default());
        let user = user_from_sheet(&found, session_id);
        let prev_fp = preset_caps_fingerprint(&current.access_level);
        let next_fp = preset_caps_fingerprint(&user.access_level);
        let changed =
            session_caps_changed(&current, &user) || prev_fp != next_fp;
        self.save_session(&user)?;
        if self.session_gen.load(Ordering::SeqCst) != gen {
            let _ = fs::remove_file(Self::session_path());
            return Err(t("errors.auth.userNotAuthorized"));
        }
        if let Ok(mut g) = self.current.lock() {
            if self.session_gen.load(Ordering::SeqCst) == gen {
                *g = Some(user.clone());
            }
        }
        if changed {
            let _ = super::super::emit_session_updated(&user);
        }
        Ok(user)
    }

    async fn revoke_live_session(
        &self,
        user: &UserInfo,
        client: &crate::core::google_sheets::GoogleSheetsClient,
        sid: &str,
    ) {
        let reason = t("errors.auth.noProjectAccess");
        self.log.warn(tf(
            "log.auth.accessRevokedKick",
            &[("login", &user.login)],
        ));
        self.session_gen.fetch_add(1, Ordering::SeqCst);
        self.stop_activity_timer();
        let _ = fs::remove_file(Self::session_path());
        if let Ok(mut g) = self.current.lock() {
            *g = None;
        }
        if user.row_index > 0 {
            if let Err(e) = clear_session_cells(client, sid, user.row_index).await {
                self.log.warn(tf(
                    "log.auth.logoutSheetClearFailed",
                    &[("error", &e)],
                ));
            }
        }
        super::super::emit_access_revoked(&reason);
    }

    pub async fn change_password(
        &self,
        current_password: &str,
        new_password: &str,
        confirm_password: &str,
    ) -> Result<(), String> {
        let user = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.userNotAuthorized"))?;

        if new_password.trim().is_empty() {
            return Err(t("errors.auth.newPasswordEmpty"));
        }
        if new_password.len() < 8 {
            return Err(t("errors.auth.newPasswordMin8"));
        }
        if new_password != confirm_password {
            return Err(t("errors.auth.passwordConfirmMismatch"));
        }

        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        find_user_by_credentials(&client, &sid, &user.login, current_password)
            .await
            .map_err(|_| t("errors.auth.currentPasswordWrong"))?;

        write_password(&client, &sid, user.row_index, new_password).await?;
        // Ротация SessionId — другие машины с тем же session.json отвалятся на activity-kick.
        let machine = machine_id();
        let session_id = format!("{}_{}", Uuid::new_v4(), machine);
        let now = Utc::now();
        write_session(&client, &sid, user.row_index, &session_id, now).await?;
        let mut next = user.clone();
        next.session_id = Some(session_id);
        if let Ok(raw) = serde_json::to_string_pretty(&super::super::SessionFile {
            login: next.login.clone(),
            display_name: next.display_name.clone(),
            access_level: next.access_level.clone(),
            row_index: next.row_index,
            session_id: next.session_id.clone(),
            birth_date: next.birth_date.clone(),
            last_login: next.last_login.clone(),
            google_email: next.google_email.clone(),
            has_oauth: next.has_oauth,
        }) {
            let _ = Self::write_session_file(&raw);
        }
        if let Ok(mut g) = self.current.lock() {
            *g = Some(next.clone());
        }
        let _ = super::super::emit_session_updated(&next);
        self.log.info(t("log.auth.passwordChanged"));
        Ok(())
    }

    pub async fn logout(&self) {
        self.session_gen.fetch_add(1, Ordering::SeqCst);
        self.stop_activity_timer();
        let user = self.current_user();
        // Локальная сессия снимается сразу, до сети: закрытие приложения
        // во время очистки листа не должно вернуть вход при следующем запуске.
        let _ = fs::remove_file(Self::session_path());
        if let Ok(mut g) = self.current.lock() {
            *g = None;
        }
        crate::calculator::article_index::clear();
        // Shared PC: следующий логин не наследует чужой Google token.
        // oauth_client.json команды оставляем — общий ресурс.
        crate::auth::team_oauth_sheet::reset_auto_pull_gate();
        google_oauth::clear_cached_token();
        self.log.info(t("log.auth.logout"));
        if let Some(user) = user.filter(|u| u.row_index > 0) {
            let log = self.log.clone();
            tauri::async_runtime::spawn(async move {
                if let (Ok(sid), Ok(client)) = (spreadsheet_id(), auth_client().await) {
                    if let Err(e) = clear_session_cells(&client, &sid, user.row_index).await {
                        log.warn(tf(
                            "log.auth.logoutSheetClearFailed",
                            &[("error", &e)],
                        ));
                    }
                }
            });
        }
    }

    fn clear_local_session_keep_files(&self) {
        self.session_gen.fetch_add(1, Ordering::SeqCst);
        self.stop_activity_timer();
        if let Ok(mut g) = self.current.lock() {
            *g = None;
        }
        crate::calculator::article_index::clear();
    }

    /// Хаб: выбрать команду без входа (сессия в RAM сбрасывается, файл другой команды не трогаем).
    pub fn set_active_team(&self, id: &str) -> Result<(), String> {
        self.clear_local_session_keep_files();
        crate::core::team_profile::set_active(id)?;
        crate::auth::team_oauth_sheet::reset_auto_pull_gate();
        Ok(())
    }

    /// Хаб: новый слот под Admin OAuth.
    pub fn begin_new_team(&self) -> Result<String, String> {
        self.clear_local_session_keep_files();
        let id = crate::core::team_profile::begin_new_team()?;
        crate::auth::team_oauth_sheet::reset_auto_pull_gate();
        Ok(id)
    }

    /// Профиль: переключить команду. Admin → Google OAuth; Member — без браузера.
    pub async fn switch_team(&self, id: &str) -> Result<super::super::SwitchTeamResult, String> {
        use crate::core::team_profile::{self, target_needs_google};
        use super::super::SwitchTeamResult;

        let id = id.trim();
        let entry = team_profile::entry(id).ok_or_else(|| t("errors.auth.teamNotFound"))?;
        self.clear_local_session_keep_files();
        team_profile::set_active(id)?;
        crate::auth::team_oauth_sheet::reset_auto_pull_gate();

        if target_needs_google(&entry) {
            if !oauth_config::is_configured() {
                return Ok(SwitchTeamResult {
                    status: "needsLogin".into(),
                    user: None,
                });
            }
            self.log.info(t("log.auth.switchTeamGoogle"));
            let _ = google_oauth::authorize_interactive().await?;
        }

        self.load_session();
        if let Some(user) = self.current_user().filter(|u| u.is_authorized) {
            self.start_activity_timer();
            self.ensure_presets_loaded().await;
            let role = if access::is_admin_role(&user.access_level) {
                "Admin"
            } else {
                "Member"
            };
            let _ = team_profile::update_last_role(role);
            return Ok(SwitchTeamResult {
                status: "ok".into(),
                user: Some(user),
            });
        }
        Ok(SwitchTeamResult {
            status: "needsLogin".into(),
            user: None,
        })
    }

}

pub(crate) fn user_from_sheet(found: &SheetUser, session_id: String) -> UserInfo {
    let has_oauth =
        access::is_admin_role(&found.access_level) || found.has_oauth;
    UserInfo {
        login: found.login.clone(),
        display_name: found.display_name(),
        is_authorized: true,
        access_level: found.access_level.clone(),
        row_index: found.row_index,
        session_id: Some(session_id),
        birth_date: String::new(),
        last_login: found.last_login_display().unwrap_or_default(),
        google_email: found.google_email.clone(),
        has_oauth,
    }
}

pub(crate) fn sheet_access_revoked(access_level: &str) -> bool {
    !access::is_admin_role(access_level) && access::find_preset(access_level).is_none()
}

pub(crate) fn session_caps_changed(prev: &UserInfo, next: &UserInfo) -> bool {
    prev.access_level != next.access_level
        || prev.has_oauth != next.has_oauth
        || prev.google_email != next.google_email
        || prev.display_name != next.display_name
}

/// Снимок флагов пресета (AppAccess / CalcAccess / Scripts) — ловит правки без смены id.
pub(crate) fn preset_caps_fingerprint(access_level: &str) -> String {
    if access::is_admin_role(access_level) {
        return "admin".into();
    }
    let Some(p) = access::find_preset(access_level) else {
        return format!("none:{}", access_level.trim().to_ascii_lowercase());
    };
    let mut calc: Vec<_> = p.calc_access.iter().cloned().collect();
    calc.sort();
    let mut app: Vec<_> = p.app_access.iter().cloned().collect();
    app.sort();
    let scripts = match &p.scripts {
        access::AccessMatrix::All => "*".into(),
        access::AccessMatrix::Keys(set) => {
            let mut keys: Vec<_> = set.iter().cloned().collect();
            keys.sort();
            keys.join(",")
        }
    };
    format!(
        "{}|calc:{}|c:{}|a:{}|s:{}",
        p.id.to_ascii_lowercase(),
        if p.calculator { "1" } else { "0" },
        calc.join(","),
        app.join(","),
        scripts
    )
}

/// Login строки Users должен совпадать с сессией (защита от подмены row_index).
/// Пустой локальный SessionId — не ok (иначе kick/сверка обходятся).
pub(crate) fn sheet_row_matches_session(user: &UserInfo, row: &SheetUser) -> bool {
    let sheet_login = row.login.trim();
    if sheet_login.is_empty()
        || !sheet_login.eq_ignore_ascii_case(user.login.trim())
    {
        return false;
    }
    let local_sid = user
        .session_id
        .as_deref()
        .unwrap_or("")
        .trim();
    if local_sid.is_empty() {
        return false;
    }
    let sheet_sid = row.session_id.as_deref().unwrap_or("").trim();
    !sheet_sid.is_empty() && sheet_sid == local_sid
}

pub(crate) fn machine_id() -> String {
    let info = format!(
        "{}_{}_{}",
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "pc".into()),
        std::env::var("USERNAME").unwrap_or_else(|_| "user".into()),
        std::env::var("USERDOMAIN").unwrap_or_else(|_| "dom".into()),
    );
    let hash = Sha256::digest(info.as_bytes());
    hex::encode(&hash[..8])
}
