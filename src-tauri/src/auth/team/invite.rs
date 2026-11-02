//! Invite team member.
use super::super::{AuthService, TeamUserRow};
use crate::calculator::team_config;
use crate::core::access::{self, ensure_access_sheets, resolve_preset_id};
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use crate::core::service_account_cfg;
use super::super::sheets_users::{
    append_user_row, auth_client, spreadsheet_id,
};

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
        let entry = team_config::resolve_entry(&catalog, calc_config_id)
            .cloned()
            .ok_or_else(|| t("errors.calc.teamNotFound"))?;
        let drive_profile = team_config::profile_for_user_drive(&preset_label, &entry.profile);
        let needs_drive = !team_config::table_ids_from_profile(&drive_profile).is_empty();
        // Fail-closed: не писать строку Users, если Drive обязателен, а OAuth менеджера нет.
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
        let name = {
            let d = display_name.trim();
            if d.is_empty() {
                login
            } else {
                d
            }
        };
        let row = append_user_row(
            &client,
            &sid,
            login,
            password,
            name,
            &preset_label,
            email,
            &entry.id,
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
            calc_config_id: entry.id,
        })
    }
}
