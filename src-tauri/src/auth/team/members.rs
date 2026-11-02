//! Delete user + member OAuth connect.
use super::super::AuthService;
use crate::calculator::team_config;
use crate::core::access;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use crate::core::google_oauth;
use super::super::sheets_users::{
    auth_client, delete_user_row, list_users, read_user_calc_config, read_user_row,
    spreadsheet_id, write_google_email, write_has_oauth,
};

impl AuthService {
    pub async fn delete_user(&self, row_index: u32) -> Result<(), String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_manage_team(&admin.access_level) {
            return Err(t("errors.auth.deleteUserAdminOnly"));
        }
        if row_index < 2 {
            return Err(t("errors.auth.invalidUserRowIndex"));
        }
        if admin.row_index == row_index {
            return Err(t("errors.auth.cannotDeleteSelf"));
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let users = list_users(&client, &sid).await?;
        let user = users
            .into_iter()
            .find(|u| u.row_index == row_index)
            .ok_or_else(|| t("errors.auth.userNotFound"))?;
        if access::is_admin_role(&user.access_level) {
            return Err(t("errors.auth.cannotDeleteAdmin"));
        }
        let email = user.google_email.trim().to_string();
        if !email.is_empty() {
            if let Ok(drive) = GoogleSheetsClient::from_user_oauth().await {
                let catalog = team_config::load_catalog(&client).await.ok();
                if let Some(cat) = catalog {
                    let cfg_id = read_user_calc_config(&client, &sid, row_index)
                        .await
                        .unwrap_or_default();
                    if let Some(entry) = team_config::resolve_entry(&cat, &cfg_id) {
                        let empty = {
                            let mut p = entry.profile.clone();
                            p.table_links.clear();
                            p
                        };
                        let title = format!("Удаление {}", user.login);
                        let _ = team_config::sync_drive_access(
                            &drive,
                            &email,
                            Some(&entry.profile),
                            &empty,
                            &self.log,
                            &title,
                            &team_config::keep_going,
                        )
                        .await;
                    }
                }
            }
        }
        delete_user_row(&client, &sid, row_index).await?;
        access::invalidate_team_cache();
        access::invalidate_users_ui_cache();
        self.log.info(tf(
            "log.auth.userDeleted",
            &[("login", &user.login), ("row", &row_index.to_string())],
        ));
        Ok(())
    }

    /// Keys Check: подключить Google (браузер) → HasOauth=да.
    /// Всегда проверяет грант TeamOAuth (даже если client уже на диске от другого логина).
    pub async fn connect_member_oauth(&self) -> Result<crate::auth::UserInfo, String> {
        let mut user = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.notAuthorized"))?;
        if access::is_admin_role(&user.access_level) {
            user.has_oauth = true;
            let _ = self.save_session(&user);
            if let Ok(mut g) = self.current.lock() {
                *g = Some(user.clone());
            }
            return Ok(user);
        }
        if user.row_index < 2 {
            return Err(t("errors.auth.invalidUserRowIndex"));
        }
        // Грант обязателен до браузера (shared PC), даже если client уже на диске.
        if crate::core::oauth_config::is_configured() {
            let allowed = crate::auth::team_oauth_sheet::refresh_can_check_google(
                &user.access_level,
                &user.login,
            )
            .await;
            if !allowed {
                return Err(t("errors.auth.teamOauthDenied"));
            }
        } else {
            match crate::auth::team_oauth_sheet::pull_if_allowed(
                &user.access_level,
                &user.login,
            )
            .await
            {
                Ok(_) => {
                    crate::auth::team_oauth_sheet::log_pull_ok(&self.log, &user.login);
                }
                Err(e) => return Err(e),
            }
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let sheet_row = read_user_row(&client, &sid, user.row_index).await?;
        let expected = sheet_row.google_email.trim().to_string();
        google_oauth::authorize_interactive().await?;
        let email = google_oauth::cached_email().unwrap_or_default();
        if email.trim().is_empty() {
            google_oauth::clear_cached_token();
            return Err(t("errors.auth.teamGoogleRequired"));
        }
        if !expected.is_empty() && !email.eq_ignore_ascii_case(&expected) {
            google_oauth::clear_cached_token();
            return Err(tf(
                "errors.auth.googleEmailMismatch",
                &[("oauthEmail", email.trim()), ("expected", &expected)],
            ));
        }
        if expected.is_empty() {
            write_google_email(&client, &sid, user.row_index, &email).await?;
        }
        write_has_oauth(&client, &sid, user.row_index, true).await?;
        user.google_email = email;
        user.has_oauth = true;
        self.save_session(&user)?;
        if let Ok(mut g) = self.current.lock() {
            *g = Some(user.clone());
        }
        // Шаблоны, сохранённые в Excel-only, иначе продолжают писать в .xlsx.
        match crate::templates::store::clear_stuck_excel_exports() {
            Ok(n) if n > 0 => self.log.info(tf(
                "log.auth.templatesExcelFlagsCleared",
                &[("count", &n.to_string())],
            )),
            Ok(_) => {}
            Err(e) => self.log.warn(tf(
                "log.auth.templatesExcelFlagsClearFailed",
                &[("error", &e)],
            )),
        }
        self.log.info(tf(
            "log.auth.memberOauthConnected",
            &[("login", &user.login)],
        ));
        Ok(user)
    }
}
