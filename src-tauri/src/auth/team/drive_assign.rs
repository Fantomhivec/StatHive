//! Calc assign gens + Drive sync for team users.
use super::super::AuthService;
use crate::calculator::team_config;
use crate::core::access;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use super::super::sheets_users::{
    auth_client, list_users, read_user_calc_config, spreadsheet_id, write_access_level,
    write_calc_config,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn assign_gens() -> &'static Mutex<HashMap<u32, u64>> {
    static GENS: OnceLock<Mutex<HashMap<u32, u64>>> = OnceLock::new();
    GENS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn begin_assign(row: u32) -> u64 {
    let mut map = assign_gens().lock().unwrap_or_else(|e| e.into_inner());
    let n = map.entry(row).or_insert(0);
    *n += 1;
    *n
}

fn assign_is_current(row: u32, gen: u64) -> bool {
    assign_gens()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&row)
        .copied()
        == Some(gen)
}

pub fn next_assign_generation(row: u32) -> u64 {
    begin_assign(row)
}

impl AuthService {
    pub async fn set_user_calc_config(
        &self,
        row_index: u32,
        calc_config_id: &str,
        gen: u64,
    ) -> Result<(), String> {
        let admin = self
            .current_user()
            .filter(|u| u.is_authorized)
            .ok_or_else(|| t("errors.auth.needAdminAuth"))?;
        if !access::can_users_tab(&admin.access_level) {
            return Err(t("errors.auth.inviteAdminOnly"));
        }
        if row_index < 2 {
            return Err(t("errors.calc.teamAssignAdminRow"));
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let users = list_users(&client, &sid).await?;
        let user = users
            .into_iter()
            .find(|u| u.row_index == row_index)
            .ok_or_else(|| t("errors.auth.userNotFound"))?;
        if access::is_admin_role(&user.access_level) {
            return Err(t("errors.calc.teamAssignAdminRow"));
        }
        let email = user.google_email.trim();
        if email.is_empty() {
            return Err(t("errors.calc.teamEmailRequired"));
        }
        let catalog = team_config::load_catalog(&client).await?;
        let new_entry = team_config::resolve_entry(&catalog, calc_config_id)
            .cloned()
            .ok_or_else(|| t("errors.calc.teamNotFound"))?;
        let old_id = read_user_calc_config(&client, &sid, row_index).await?;
        let old_resolved = team_config::resolve_entry(&catalog, &old_id)
            .map(|e| e.id.clone())
            .unwrap_or_default();
        let title = format!("Назначение {}", user.login);
        if !assign_is_current(row_index, gen) {
            self.log.info(tf("log.auth.assignCancelled", &[("title", &title)]));
            return Ok(());
        }
        if new_entry.id.eq_ignore_ascii_case(&old_resolved) {
            self.log.info(tf("log.auth.assignUnchanged", &[("title", &title)]));
            return Ok(());
        }
        let old_profile = team_config::resolve_entry(&catalog, &old_id).map(|e| e.profile.clone());
        let old_drive = old_profile
            .as_ref()
            .map(|p| team_config::profile_for_user_drive(&user.access_level, p));
        let new_drive =
            team_config::profile_for_user_drive(&user.access_level, &new_entry.profile);
        let old_ids = old_drive
            .as_ref()
            .map(team_config::table_ids_from_profile)
            .unwrap_or_default();
        let new_ids = team_config::table_ids_from_profile(&new_drive);
        let needs_drive = !old_ids.is_empty() || !new_ids.is_empty();
        if needs_drive {
            let drive = GoogleSheetsClient::from_user_oauth().await.map_err(|e| {
                self.log.error(format!("{title}: Drive — {e}"));
                t("errors.auth.inviteNeedsManagerOauth")
            })?;
            let stop = move || !assign_is_current(row_index, gen);
            if !new_ids.is_empty() {
                team_config::share_tables_with_service_account(
                    &drive,
                    &new_entry.profile,
                    &self.log,
                    &title,
                    &stop,
                )
                .await?;
            }
            if stop() {
                return Ok(());
            }
            team_config::sync_drive_access(
                &drive,
                email,
                old_drive.as_ref(),
                &new_drive,
                &self.log,
                &title,
                &stop,
            )
            .await?;
            if stop() {
                return Ok(());
            }
        }
        write_calc_config(&client, &sid, row_index, &new_entry.id).await?;
        access::invalidate_team_cache();
        self.log.info(tf(
            "log.calc.teamAssigned",
            &[
                ("login", &user.login),
                ("name", &new_entry.name),
                ("id", &new_entry.id),
            ],
        ));
        Ok(())
    }

    pub async fn clear_users_after_preset_delete(&self, preset_id: &str) -> Result<(), String> {
        let id = preset_id.trim();
        if id.is_empty() {
            return Ok(());
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let users = list_users(&client, &sid).await?;
        let affected: Vec<_> = users
            .into_iter()
            .filter(|u| {
                !access::is_admin_role(&u.access_level)
                    && u.access_level.trim().eq_ignore_ascii_case(id)
            })
            .collect();
        if affected.is_empty() {
            access::invalidate_team_cache();
            return Ok(());
        }
        let catalog = team_config::load_catalog(&client).await.ok();
        let drive = GoogleSheetsClient::from_user_oauth().await.ok();
        for u in &affected {
            if let Err(e) = write_access_level(&client, &sid, u.row_index, "").await {
                self.log.warn(format!(
                    "Пресет удалён: AccessLevel row={} — {e}",
                    u.row_index
                ));
                continue;
            }
            let email = u.google_email.trim();
            if email.is_empty() {
                continue;
            }
            let (Some(drive), Some(cat)) = (drive.as_ref(), catalog.as_ref()) else {
                continue;
            };
            let cfg_id = read_user_calc_config(&client, &sid, u.row_index)
                .await
                .unwrap_or_default();
            let Some(entry) = team_config::resolve_entry(cat, &cfg_id) else {
                continue;
            };
            let old_drive = team_config::profile_for_user_drive(id, &entry.profile);
            let empty = {
                let mut p = entry.profile.clone();
                p.table_links.clear();
                p
            };
            let title = format!("Удаление пресета {}", u.login);
            if let Err(e) = team_config::sync_drive_access(
                drive,
                email,
                Some(&old_drive),
                &empty,
                &self.log,
                &title,
                &team_config::keep_going,
            )
            .await
            {
                self.log.warn(format!("{title}: Drive — {e}"));
            }
        }
        access::invalidate_team_cache();
        access::invalidate_users_ui_cache();
        Ok(())
    }
    pub async fn resync_drive_for_user_row(
        &self,
        row_index: u32,
        old_access_level: &str,
    ) -> Result<(), String> {
        if row_index < 2 {
            return Ok(());
        }
        let sid = spreadsheet_id()?;
        let client = auth_client().await?;
        let users = list_users(&client, &sid).await?;
        let user = users
            .into_iter()
            .find(|u| u.row_index == row_index)
            .ok_or_else(|| t("errors.auth.userNotFound"))?;
        if access::is_admin_role(&user.access_level) {
            return Ok(());
        }
        let email = user.google_email.trim();
        if email.is_empty() {
            return Ok(());
        }
        let catalog = team_config::load_catalog(&client).await?;
        let cfg_id = read_user_calc_config(&client, &sid, row_index).await?;
        let entry = team_config::resolve_entry(&catalog, &cfg_id)
            .cloned()
            .ok_or_else(|| t("errors.calc.teamNotFound"))?;
        let old_drive =
            team_config::profile_for_user_drive(old_access_level, &entry.profile);
        let new_drive =
            team_config::profile_for_user_drive(&user.access_level, &entry.profile);
        let old_ids = team_config::table_ids_from_profile(&old_drive);
        let new_ids = team_config::table_ids_from_profile(&new_drive);
        if old_ids == new_ids {
            return Ok(());
        }
        let drive = GoogleSheetsClient::from_user_oauth().await?;
        let title = format!("Пресет {}", user.login);
        team_config::sync_drive_access(
            &drive,
            email,
            Some(&old_drive),
            &new_drive,
            &self.log,
            &title,
            &team_config::keep_going,
        )
        .await?;
        Ok(())
    }
}
