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

//! Login and finish_login.
use super::super::super::{AuthService, UserInfo};
use super::helpers::{is_sheets_transport_error, machine_id, user_from_sheet};
use crate::core::access::{self, reload_presets_cache};
use crate::core::google_oauth;
use crate::core::i18n::{t, tf};
use crate::core::workspace_cfg;
use chrono::Utc;
use std::sync::atomic::Ordering;
use uuid::Uuid;
use super::super::super::sheets_users::{
    auth_client, find_user_by_credentials, find_user_by_credentials_scan,
    has_active_foreign_session, spreadsheet_id, write_last_login, write_password,
    write_session, SheetUser,
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
        let client = match auth_client().await {
            Ok(c) => c,
            Err(e) => {
                self.log.warn(tf(
                    "log.auth.loginCredentialsFailed",
                    &[("error", &e)],
                ));
                return Err(if is_sheets_transport_error(&e) {
                    t("errors.auth.sheetsUnavailable")
                } else {
                    t("errors.auth.rejected")
                });
            }
        };
        // Один GET A2:J1000: credentials + email Admin (раньше find + list_users).
        let (found, admin_mail, needs_rehash) =
            find_user_by_credentials_scan(&client, &sid, login, password)
                .await
                .map_err(|e| {
                    self.log.warn(tf(
                        "log.auth.loginCredentialsFailed",
                        &[("error", &e.to_string())],
                    ));
                    // Не светить userNotFound / детали — credential oracle.
                    if is_sheets_transport_error(&e) {
                        t("errors.auth.sheetsUnavailable")
                    } else {
                        t("errors.auth.rejected")
                    }
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
            return Err(t("errors.auth.rejected"));
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
                return Err(t("errors.auth.rejected"));
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
        if let Err(e) = write_session(client, sid, found.row_index, &session_id, now).await {
            self.log.warn(tf(
                "log.auth.loginSessionWriteFailed",
                &[("error", &e), ("attempt", "1")],
            ));
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if let Err(e2) = write_session(client, sid, found.row_index, &session_id, now).await {
                self.log.warn(tf(
                    "log.auth.loginSessionWriteFailed",
                    &[("error", &e2), ("attempt", "2")],
                ));
                return Err(t("errors.auth.sheetsUnavailable"));
            }
        }
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
}
