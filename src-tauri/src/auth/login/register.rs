//! Register first Admin + workspace sheet bootstrap.
use super::super::{AuthService, UserInfo};
use crate::core::access::ensure_access_sheets;
use crate::core::i18n::{t, tf};
use crate::core::google_oauth;
use crate::core::oauth_config;
use crate::core::workspace_cfg;
use super::super::sheets_users::{
    append_user_row, user_client, write_header, USERS_SHEET_NAME,
};
use crate::core::access::RoleId;

impl AuthService {
    pub async fn register_admin(
        &self,
        login: &str,
        password: &str,
        display_name: &str,
        workspace_title: &str,
    ) -> Result<UserInfo, String> {
        let login = login.trim();
        if login.is_empty() || password.is_empty() {
            return Err(t("errors.auth.enterLoginPassword"));
        }
        if password.len() < 8 {
            return Err(t("errors.auth.passwordMin8"));
        }
        // Active-слот должен быть пустым (begin_new_team) или без spreadsheet.
        if workspace_cfg::is_configured() {
            return Err(t("errors.auth.workspaceAlreadyOnPc"));
        }
        if !oauth_config::is_configured() {
            return Err(t("errors.auth.saveOAuthFirst"));
        }
        self.log.info(tf(
            "log.auth.registerAdminStart",
            &[("login", login)],
        ));
        if !google_oauth::has_cached_token() {
            self.log.info(t("log.auth.registerAdminOAuthBrowser"));
            let _ = google_oauth::authorize_interactive().await?;
        }
        let email = google_oauth::cached_email().unwrap_or_default();
        self.log.info(tf("log.auth.registerAdminGoogle", &[("email", &email)]));
        let client = user_client().await?;
        let title = workspace_cfg::normalize_workspace_title(workspace_title);
        self.log.info(tf(
            "log.auth.registerAdminCreateSpreadsheet",
            &[("title", &title)],
        ));
        let sid = client
            .create_spreadsheet(&title, USERS_SHEET_NAME)
            .await
            .map_err(|e| {
                self.log.error(tf(
                    "log.auth.registerAdminCreateSpreadsheetError",
                    &[("error", &e.to_string())],
                ));
                e
            })?;
        self.log.info(tf(
            "log.auth.registerAdminSpreadsheetId",
            &[("id", &sid)],
        ));
        write_header(&client, &sid).await.map_err(|e| {
            self.log.error(tf(
                "log.auth.registerAdminSheetHeaderError",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        self.log.info(t("log.auth.registerAdminUsersHeader"));
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
            RoleId::Admin.as_str(),
            &email,
            "",
        )
        .await
        .map_err(|e| {
            self.log.error(tf(
                "log.auth.registerAdminUserRowError",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        let row_s = row.to_string();
        self.log.info(tf(
            "log.auth.registerAdminUserRow",
            &[("row", &row_s), ("login", login)],
        ));
        workspace_cfg::set_spreadsheet(&sid, &title)?;
        let _ = workspace_cfg::set_joined_by_code(false);
        let _ = crate::core::team_profile::finalize_new_team(&sid, &title);
        self.log.info(tf(
            "log.auth.registerAdminWorkspaceSaved",
            &[("id", &sid)],
        ));
        ensure_access_sheets(&client, &sid).await.map_err(|e| {
            self.log.error(tf(
                "log.auth.registerAdminRolesPresetsError",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        self.log.info(t("log.auth.registerAdminRolesPresetsReady"));

        self.login_with_client(&client, &sid, login, password, row)
            .await
    }
}
