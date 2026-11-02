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

//! Invite team member.
use super::super::sheets_users::{append_user_row, auth_client, spreadsheet_id};
use super::super::{AuthService, TeamUserRow};
use crate::calculator::team_config;
use crate::core::access::{self, ensure_access_sheets, resolve_preset_id};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use crate::core::service_account_cfg;

impl AuthService {
    pub async fn invite_user(
        &self,
        login: &str,
        password: &str,
        display_name: &str,
        google_email: &str,
        preset_id: &str,
        calc_config_id: &str,
    ) -> Result<TeamUserRow, String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_users_tab(&admin.access_level) {
            self.log.warn(tf(
                "log.auth.inviteRejectedNotAdmin",
                &[
                    ("login", &admin.login),
                    ("accessLevel", &admin.access_level),
                ],
            ));
            return Err(t("errors.auth.inviteAdminOnly"));
        }
        let login = login.trim();
        let email = google_email.trim();
        if login.is_empty() || password.is_empty() || email.is_empty() {
            return Err(t("errors.auth.inviteFieldsRequired"));
        }
        if password.len() < 8 {
            return Err(t("errors.auth.passwordMin8"));
        }
        if preset_id.trim().is_empty() {
            return Err(t("errors.auth.chooseInvitePreset"));
        }
        if !service_account_cfg::is_configured() {
            return Err(t("errors.auth.saveServiceAccountFirst"));
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await.map_err(|e| {
            self.log.error(tf(
                "log.auth.inviteOAuthSheetsError",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        let _ = ensure_access_sheets(&client, &sid).await;
        let preset_label = resolve_preset_id(&client, &sid, preset_id).await?;
        let catalog = team_config::load_catalog(&client).await?;
        // Конфиг калькулятора опционален, если каталог пуст; иначе нужен валидный id / default.
        let entry = team_config::resolve_entry(&catalog, calc_config_id).cloned();
        if entry.is_none()
            && (!catalog.configs.is_empty() || !calc_config_id.trim().is_empty())
        {
            return Err(t("errors.calc.teamNotFound"));
        }

        let name = {
            let d = display_name.trim();
            if d.is_empty() {
                login
            } else {
                d
            }
        };
        let calc_id_write = entry
            .as_ref()
            .map(|e| e.id.as_str())
            .unwrap_or("");

        if let Some(ref entry) = entry {
            let drive_profile = team_config::profile_for_user_drive(&preset_label, &entry.profile);
            let needs_drive = !team_config::table_ids_from_profile(&drive_profile).is_empty();
            if needs_drive {
                let _ = GoogleSheetsClient::from_user_oauth().await.map_err(|e| {
                    self.log.error(tf(
                        "log.auth.inviteDriveFailed",
                        &[("login", login), ("error", &e)],
                    ));
                    t("errors.auth.inviteNeedsManagerOauth")
                })?;
            }
            let tables_n = team_config::table_ids_from_profile(&entry.profile)
                .len()
                .to_string();
            let title = format!("Приглашение {login}");
            self.log.info(tf(
                "log.auth.inviteBrief",
                &[
                    ("login", login),
                    ("email", email),
                    ("preset", &preset_label),
                    ("name", &entry.name),
                    ("tables", &tables_n),
                ],
            ));

            let row = append_user_row(
                &client,
                &sid,
                login,
                password,
                name,
                &preset_label,
                email,
                calc_id_write,
            )
            .await
            .map_err(|e| {
                self.log.error(tf(
                    "log.auth.inviteAppendFailed",
                    &[("error", &e.to_string())],
                ));
                e
            })?;
            let row_s = row.to_string();
            self.log.info(tf(
                "log.auth.inviteRow",
                &[("login", login), ("row", &row_s)],
            ));
            access::invalidate_team_cache();
            match GoogleSheetsClient::from_user_oauth().await {
                Ok(drive) => {
                    if let Err(e) = team_config::share_tables_with_service_account(
                        &drive,
                        &entry.profile,
                        &self.log,
                        &title,
                        &team_config::keep_going,
                    )
                    .await
                    {
                        self.log.error(tf(
                            "log.auth.inviteSaShareFailed",
                            &[("login", login), ("error", &e)],
                        ));
                        if needs_drive {
                            return Err(e);
                        }
                    }
                    if let Err(e) = team_config::sync_drive_access(
                        &drive,
                        email,
                        None,
                        &drive_profile,
                        &self.log,
                        &title,
                        &team_config::keep_going,
                    )
                    .await
                    {
                        self.log.error(tf(
                            "log.auth.inviteDriveFailed",
                            &[("login", login), ("error", &e)],
                        ));
                        if needs_drive {
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    self.log.error(tf(
                        "log.auth.inviteDriveFailed",
                        &[("login", login), ("error", &e)],
                    ));
                    if needs_drive {
                        return Err(e);
                    }
                }
            }
            self.log.info(tf("log.auth.inviteDoneShort", &[("login", login)]));
            Ok(TeamUserRow {
                login: login.to_string(),
                display_name: name.to_string(),
                access_level: preset_label,
                google_email: email.to_string(),
                row_index: row,
                calc_config_id: entry.id.clone(),
            })
        } else {
            self.log.info(tf(
                "log.auth.inviteBrief",
                &[
                    ("login", login),
                    ("email", email),
                    ("preset", &preset_label),
                    ("name", "—"),
                    ("tables", "0"),
                ],
            ));
            let row = append_user_row(
                &client,
                &sid,
                login,
                password,
                name,
                &preset_label,
                email,
                "",
            )
            .await
            .map_err(|e| {
                self.log.error(tf(
                    "log.auth.inviteAppendFailed",
                    &[("error", &e.to_string())],
                ));
                e
            })?;
            let row_s = row.to_string();
            self.log.info(tf(
                "log.auth.inviteRow",
                &[("login", login), ("row", &row_s)],
            ));
            access::invalidate_team_cache();
            self.log.info(tf("log.auth.inviteDoneShort", &[("login", login)]));
            Ok(TeamUserRow {
                login: login.to_string(),
                display_name: name.to_string(),
                access_level: preset_label,
                google_email: email.to_string(),
                row_index: row,
                calc_config_id: String::new(),
            })
        }
    }
}
