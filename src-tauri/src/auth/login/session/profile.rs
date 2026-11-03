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

//! Refresh profile, password change, logout.
use super::super::super::{AuthService, UserInfo};
use super::helpers::{
    machine_id, preset_caps_fingerprint, session_caps_changed, sheet_access_revoked,
    sheet_row_matches_session, user_from_sheet,
};
use crate::core::access::reload_presets_cache;
use crate::core::google_oauth;
use crate::core::i18n::{t, tf};
use chrono::Utc;
use std::fs;
use std::sync::atomic::Ordering;
use uuid::Uuid;
use super::super::super::sheets_users::{
    auth_client, clear_session_cells, find_user_by_credentials, read_user_row, spreadsheet_id,
    write_password, write_session,
};

impl AuthService {
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
            let _ = super::super::super::emit_session_updated(&user);
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
        super::super::super::emit_access_revoked(&reason);
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
        if let Ok(raw) = serde_json::to_string_pretty(&super::super::super::SessionFile {
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
        let _ = super::super::super::emit_session_updated(&next);
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
}
