//! Users dashboard snapshot.
use super::super::{AuthService, TeamUserRow, UsersDashboard};
use crate::core::access::{self, ensure_access_sheets};
use crate::core::i18n::{t, tf};
use super::super::sheets_users::{auth_client, list_users, spreadsheet_id};
use std::sync::OnceLock;

fn dash_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

impl AuthService {
    pub async fn list_team(&self) -> Result<Vec<TeamUserRow>, String> {
        let dash = self.users_dashboard(false).await?;
        Ok(dash.team)
    }

    /// Снимок вкладки Пользователи: кэш в RAM, Sheets только при промахе или force.
    pub async fn users_dashboard(&self, force: bool) -> Result<UsersDashboard, String> {
        let _gate = dash_lock().lock().await;
        let user = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.notAuthorized"))?;
        if !access::can_users_tab(&user.access_level) {
            return Err(t("errors.auth.teamListAdminOnly"));
        }
        if force {
            access::invalidate_users_ui_cache();
        }
        let setup = self.setup_status();
        if !force && access::users_ui_cache_ready() {
            let team = access::cached_team()
                .unwrap_or_default()
                .into_iter()
                .map(|r| TeamUserRow {
                    login: r.login,
                    display_name: r.display_name,
                    access_level: r.access_level,
                    google_email: r.google_email,
                    row_index: r.row_index,
                    calc_config_id: r.calc_config_id,
                })
                .collect();
            let presets = access::cached_presets();
            let sa_email = if access::can_manage_team(&user.access_level) {
                setup.service_account_email
            } else {
                None
            };
            return Ok(UsersDashboard {
                team,
                presets,
                workspace_id: setup.workspace_id,
                workspace_title: setup.workspace_title,
                service_account_configured: setup.service_account_configured,
                service_account_email: sa_email,
            });
        }

        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        ensure_access_sheets(&client, &sid).await?;
        let rows = list_users(&client, &sid).await.map_err(|e| {
            self.log.error(tf(
                "log.auth.teamReadFailed",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        let count = rows.len().to_string();
        self.log.info(tf("log.auth.teamUserCount", &[("count", &count)]));
        let team: Vec<TeamUserRow> = rows
            .into_iter()
            .map(|u| {
                let display_name = u.display_name();
                TeamUserRow {
                    login: u.login,
                    display_name,
                    access_level: u.access_level,
                    google_email: u.google_email,
                    row_index: u.row_index,
                    calc_config_id: u.calc_config_id,
                }
            })
            .collect();
        access::set_team_cache(
            team.iter()
                .map(|r| access::TeamCacheRow {
                    login: r.login.clone(),
                    display_name: r.display_name.clone(),
                    access_level: r.access_level.clone(),
                    google_email: r.google_email.clone(),
                    row_index: r.row_index,
                    calc_config_id: r.calc_config_id.clone(),
                })
                .collect(),
        );
        let presets = access::cached_presets();
        let setup = self.setup_status();
        let sa_email = if access::can_manage_team(&user.access_level) {
            setup.service_account_email
        } else {
            None
        };
        Ok(UsersDashboard {
            team,
            presets,
            workspace_id: setup.workspace_id,
            workspace_title: setup.workspace_title,
            service_account_configured: setup.service_account_configured,
            service_account_email: sa_email,
        })
    }
}
