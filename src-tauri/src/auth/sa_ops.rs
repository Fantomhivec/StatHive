//! Service Account create/replace + team invite code.
use super::AuthService;
use crate::core::access::{self};
use crate::core::google::service_account as google_sa;
use crate::core::i18n::{t, tf};
use crate::core::oauth_config;
use crate::core::service_account_cfg;
use crate::core::team_invite;
use crate::core::workspace_cfg;
use super::sheets_users::{spreadsheet_id, user_client};

impl AuthService {
    /// Сохранить Service Account и выдать ему writer на workspace (не людям).
    pub async fn save_service_account(&self, raw_json: &str) -> Result<String, String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_manage_team(&admin.access_level) {
            return Err(t("errors.auth.saAdminOnly"));
        }
        let old = service_account_cfg::client_email();
        self.persist_service_account(raw_json, old.as_deref()).await
    }

    /// Создать SA в GCP-проекте OAuth Desktop и расшарить **только** текущий workspace.
    pub async fn create_workspace_service_account(&self) -> Result<String, String> {
        self.require_sa_admin()?;
        if !oauth_config::is_configured() {
            return Err(t("errors.auth.saveOAuthFirst"));
        }
        if service_account_cfg::is_configured() {
            return Err(t("errors.auth.saAlreadyConfiguredUseReplace"));
        }
        let sid = spreadsheet_id()?;
        self.log.info(tf(
            "log.auth.saCreateStart",
            &[("workspaceId", &sid)],
        ));
        let key_json = crate::core::google::iam_sa::create_key_json_for_workspace(&sid)
            .await
            .map_err(|e| {
                self.log.error(tf(
                    "log.auth.saCreateFailed",
                    &[("error", &e.to_string())],
                ));
                e
            })?;
        let email = self.persist_service_account(&key_json, None).await?;
        self.log.info(tf("log.auth.saCreateDone", &[("email", &email)]));
        Ok(email)
    }

    /// Замена при компрометации: новый SA, share на workspace, снять Drive у старого, удалить старый SA в GCP.
    pub async fn replace_workspace_service_account(&self) -> Result<String, String> {
        self.require_sa_admin()?;
        if !oauth_config::is_configured() {
            return Err(t("errors.auth.saveOAuthFirst"));
        }
        let sid = spreadsheet_id()?;
        let old_email = service_account_cfg::client_email();
        let old_s = old_email.clone().unwrap_or_default();
        self.log.info(tf(
            "log.auth.saReplaceStart",
            &[("workspaceId", &sid), ("oldEmail", &old_s)],
        ));
        let key_json = crate::core::google::iam_sa::rotate_key_json_for_workspace(
            &sid,
            old_email.as_deref(),
        )
        .await
        .map_err(|e| {
            self.log.error(tf(
                "log.auth.saReplaceFailed",
                &[("error", &e.to_string())],
            ));
            e
        })?;
        let email = self
            .persist_service_account(&key_json, old_email.as_deref())
            .await?;
        self.log.info(tf(
            "log.auth.saReplaceDone",
            &[("email", &email), ("oldEmail", &old_s)],
        ));
        Ok(email)
    }

    fn require_sa_admin(&self) -> Result<(), String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_manage_team(&admin.access_level) {
            return Err(t("errors.auth.saAdminOnly"));
        }
        Ok(())
    }

    async fn persist_service_account(
        &self,
        raw_json: &str,
        revoke_old_email: Option<&str>,
    ) -> Result<String, String> {
        service_account_cfg::save_raw(raw_json)?;
        google_sa::clear_token_cache();
        access::invalidate_users_ui_cache();
        let email = service_account_cfg::client_email()
            .ok_or_else(|| t("errors.auth.saEmailMissing"))?;
        self.log.info(tf("log.auth.saSaved", &[("email", &email)]));
        // Новый ключ IAM: token endpoint может кратко отвечать Invalid JWT Signature.
        google_sa::wait_until_usable(raw_json)
            .await
            .map_err(|e| {
                self.log.error(tf(
                    "log.auth.saTokenWaitFailed",
                    &[("email", &email), ("error", &e.to_string())],
                ));
                e
            })?;
        let sid = spreadsheet_id()?;
        let client = user_client().await?;
        match client.share_writer_silent(&sid, &email).await {
            Ok(()) => {
                self.log.info(tf(
                    "log.auth.saDriveShareOk",
                    &[("email", &email), ("workspaceId", &sid)],
                ));
            }
            Err(e) => {
                self.log.error(tf(
                    "log.auth.saDriveShareFailed",
                    &[("email", &email), ("error", &e.to_string())],
                ));
                return Err(tf(
                    "errors.auth.saDriveShareFailed",
                    &[("error", &e.to_string())],
                ));
            }
        }
        if let Some(old) = revoke_old_email.map(str::trim).filter(|s| !s.is_empty()) {
            if !old.eq_ignore_ascii_case(&email) {
                match client.remove_user_access(&sid, old).await {
                    Ok(()) => {
                        self.log.info(tf(
                            "log.auth.saDriveRevokeOk",
                            &[("email", old), ("workspaceId", &sid)],
                        ));
                    }
                    Err(e) => {
                        // Новый SA уже работает; отзыв — best-effort.
                        self.log.warn(tf(
                            "log.auth.saDriveRevokeFailed",
                            &[("email", old), ("error", &e.to_string())],
                        ));
                    }
                }
            }
        }
        Ok(email)
    }

    /// Переименовать команду (= название Google-таблицы) — только main Admin.
    pub async fn rename_workspace(&self, new_title: &str) -> Result<String, String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::is_admin_role(&admin.access_level) {
            return Err(t("errors.auth.needAdminAuth"));
        }
        let sid = spreadsheet_id()?;
        let title = workspace_cfg::normalize_workspace_title(new_title);
        let client = user_client().await?;
        client.rename_drive_file(&sid, &title).await?;
        workspace_cfg::set_spreadsheet(&sid, &title)?;
        let _ = crate::core::team_profile::update_active_title(&title);
        self.log.info(tf(
            "log.auth.workspaceRenamed",
            &[("title", &title), ("id", &sid)],
        ));
        Ok(title)
    }

    /// Код UR3: workspace + SA под паролем (OAuth не входит).
    pub fn export_team_invite_code(&self, passphrase: &str) -> Result<String, String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_manage_team(&admin.access_level) {
            return Err(t("errors.auth.teamCodeAdminOnly"));
        }
        let sa = service_account_cfg::load_raw()
            .ok_or_else(|| t("errors.auth.saveServiceAccountFirst"))?;
        let ws = workspace_cfg::load();
        if ws.spreadsheet_id.trim().is_empty() {
            return Err(t("errors.auth.workspaceNotConfigured"));
        }
        let code = team_invite::encode(&ws.spreadsheet_id, &ws.title, &sa, passphrase)?;
        let len = code.len().to_string();
        self.log.info(tf(
            "log.auth.teamCodeBuilt",
            &[
                ("workspaceId", &ws.spreadsheet_id),
                ("length", &len),
            ],
        ));
        Ok(code)
    }

    /// Участник: UR3 + пароль → upsert SA + workspace (без oauth_client).
    pub fn apply_team_invite_code(&self, code: &str, passphrase: &str) -> Result<(), String> {
        let payload = team_invite::decode(code, passphrase)?;
        let sa_raw = serde_json::to_string(&payload.service_account)
            .map_err(|e| tf("errors.auth.saInCode", &[("error", &e.to_string())]))?;
        let id = crate::core::team_profile::upsert_from_invite(
            &payload.workspace_id,
            &payload.title,
            &sa_raw,
        )?;
        google_sa::clear_token_cache();
        self.log.info(tf(
            "log.auth.teamCodeApplied",
            &[("workspaceId", &payload.workspace_id)],
        ));
        let _ = id;
        Ok(())
    }
}
