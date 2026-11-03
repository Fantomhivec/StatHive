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

//! Hub: switch / create / remove local team profiles.
use super::super::super::AuthService;
use crate::core::access;
use crate::core::google_oauth;
use crate::core::i18n::t;
use crate::core::oauth_config;
use std::sync::atomic::Ordering;

impl AuthService {
    fn clear_local_session_keep_files(&self) {
        self.session_gen.fetch_add(1, Ordering::SeqCst);
        self.stop_activity_timer();
        if let Ok(mut g) = self.current.lock() {
            *g = None;
        }
        crate::calculator::article_index::clear();
        // Логи процесса и журнал калькулятора — не тащить между командами.
        self.log.clear();
        crate::calculator::run_log::clear();
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

    /// Удалить локальный профиль команды с ПК (в т.ч. OAuth client/token).
    pub fn remove_local_team(&self, id: &str) -> Result<(), String> {
        let id = id.trim();
        let was_active = crate::core::team_profile::active_id()
            .as_deref()
            .map(|a| a == id)
            .unwrap_or(false);
        if was_active {
            self.clear_local_session_keep_files();
        }
        crate::core::team_profile::remove_team(id)?;
        crate::auth::team_oauth_sheet::reset_auto_pull_gate();
        Ok(())
    }

    /// Профиль: переключить команду. Admin → Google OAuth; Member — без браузера.
    pub async fn switch_team(&self, id: &str) -> Result<super::super::super::SwitchTeamResult, String> {
        use crate::core::team_profile::{self, target_needs_google};
        use super::super::super::SwitchTeamResult;

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
