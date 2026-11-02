//! Каталог конфигураций калькулятора — лист `CalcTeam` в workspace (SA).

use crate::calculator::profile::CalculatorProfile;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};


use super::types::{load_stamp, table_ids_from_profile, TeamConfigEntry};

pub async fn sync_drive_access(
    drive: &GoogleSheetsClient,
    email: &str,
    old_profile: Option<&CalculatorProfile>,
    new_profile: &CalculatorProfile,
    log: &crate::core::log::AppLog,
    title: &str,
    stop: &(dyn Fn() -> bool + Send + Sync),
) -> Result<(), String> {
    let email = email.trim();
    if email.is_empty() {
        return Err(t("errors.calc.teamEmailRequired"));
    }
    let old_ids = old_profile
        .map(table_ids_from_profile)
        .unwrap_or_default();
    let new_ids = table_ids_from_profile(new_profile);
    let revoke: Vec<String> = old_ids.difference(&new_ids).cloned().collect();
    let grant: Vec<String> = new_ids.difference(&old_ids).cloned().collect();
    if revoke.is_empty() && grant.is_empty() {
        return Ok(());
    }
    let revoke_n = revoke.len().to_string();
    let grant_n = grant.len().to_string();
    log.info(tf(
        "log.auth.accessUser",
        &[
            ("title", title),
            ("email", email),
            ("grant", &grant_n),
            ("revoke", &revoke_n),
        ],
    ));
    let mut errors: Vec<String> = Vec::new();
    for id in &revoke {
        if stop() {
            log.info(tf("log.auth.assignCancelled", &[("title", title)]));
            return Ok(());
        }
        if let Err(e) = drive.remove_user_access(id, email).await {
            let msg = tf("errors.calc.teamDriveRevoke", &[("id", id), ("error", &e)]);
            log.warn(format!("{title}: {msg}"));
            errors.push(msg);
        }
    }
    for id in &grant {
        if stop() {
            log.info(tf("log.auth.assignCancelled", &[("title", title)]));
            return Ok(());
        }
        if let Err(e) = drive.share_reader_silent(id, email).await {
            let msg = tf("errors.calc.teamDriveShare", &[("id", id), ("error", &e)]);
            log.warn(format!("{title}: {msg}"));
            errors.push(msg);
        }
    }
    if errors.is_empty() {
        log.info(tf(
            "log.auth.accessUserDone",
            &[("title", title), ("email", email)],
        ));
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Reader для Service Account. В лог — старт и итог, не каждая таблица.
pub async fn share_tables_with_service_account(
    drive: &GoogleSheetsClient,
    profile: &CalculatorProfile,
    log: &crate::core::log::AppLog,
    title: &str,
    stop: &(dyn Fn() -> bool + Send + Sync),
) -> Result<(), String> {
    let Some(email) = crate::core::service_account_cfg::client_email() else {
        return Err(t("errors.auth.saveServiceAccountFirst"));
    };
    let email = email.trim().to_string();
    if email.is_empty() {
        return Err(t("errors.auth.saveServiceAccountFirst"));
    }
    let ids: Vec<String> = table_ids_from_profile(profile).into_iter().collect();
    if ids.is_empty() {
        log.info(tf("log.auth.accessSaEmpty", &[("title", title)]));
        return Ok(());
    }
    let total = ids.len().to_string();
    log.info(tf(
        "log.auth.accessSa",
        &[("title", title), ("tables", &total)],
    ));
    let mut errors = Vec::new();
    for id in &ids {
        if stop() {
            log.info(tf("log.auth.assignCancelled", &[("title", title)]));
            return Ok(());
        }
        if let Err(e) = drive.share_reader_silent(id, &email).await {
            let msg = tf("errors.calc.teamDriveShare", &[("id", id), ("error", &e)]);
            log.warn(format!("{title}: {msg}"));
            errors.push(msg);
        }
    }
    if errors.is_empty() {
        log.info(tf("log.auth.accessSaDone", &[("title", title), ("tables", &total)]));
        Ok(())
    } else {
        let n = errors.len().to_string();
        log.warn(tf(
            "log.auth.accessSaFailed",
            &[("title", title), ("count", &n), ("tables", &total)],
        ));
        Err(errors.join("; "))
    }
}

pub fn keep_going() -> bool {
    false
}

/// Нужно ли обновить локальный профиль по метке.
pub fn needs_apply(entry: &TeamConfigEntry) -> bool {
    let stamp = load_stamp();
    !stamp.config_id.eq_ignore_ascii_case(&entry.id)
        || stamp.revision.trim() != entry.revision.trim()
}

#[cfg(test)]
mod tests {
    use crate::calculator::team_config::catalog::parse_catalog;
    use crate::calculator::team_config::types::{chunk_json, CHUNK_CHARS};

    #[test]
    fn chunks_respect_char_boundary() {
        let s = "я".repeat(CHUNK_CHARS / 2 + 10);
        let parts = chunk_json(&s);
        assert!(!parts.is_empty());
        assert_eq!(parts.join(""), s);
        for p in &parts {
            assert!(p.len() <= CHUNK_CHARS + 4);
        }
    }

    #[test]
    fn parses_legacy_profile() {
        let raw = r#"{"calculatorName":"Тест","tableLinks":{}}"#;
        let cat = parse_catalog(raw).unwrap();
        assert_eq!(cat.configs.len(), 1);
        assert_eq!(cat.configs[0].name, "Тест");
        assert!(!cat.default_id.is_empty());
    }
}
