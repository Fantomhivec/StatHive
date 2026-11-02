//! Чтение листа «Пользователи».

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};
use crate::core::service_account_cfg;
use crate::core::workspace_cfg;

use super::model::{cell, parse_user, SheetUser, USERS_SHEET_NAME};

pub fn spreadsheet_id() -> Result<String, String> {
    workspace_cfg::spreadsheet_id()
        .ok_or_else(|| t("errors.auth.workspaceNotConfiguredJoin"))
}

pub async fn user_client() -> Result<GoogleSheetsClient, String> {
    GoogleSheetsClient::from_user_oauth()
        .await
        .map_err(|e| tf("errors.auth.sheetsConnectFailed", &[("error", &e.to_string())]))
}

/// Клиент для листа «Пользователи» / пресетов.
/// Участник (joined_by_code): только Service Account — без user Google OAuth.
/// Admin на ПК создания команды: SA если есть, иначе user OAuth.
pub async fn auth_client() -> Result<GoogleSheetsClient, String> {
    let joined = crate::core::workspace_cfg::load().joined_by_code;
    match service_account_cfg::load_raw_result() {
        Ok(Some(raw)) => {
            service_account_cfg::parse_key(&raw).map_err(|e| {
                tf(
                    "errors.auth.sheetsConnectFailed",
                    &[("error", &e)],
                )
            })?;
            GoogleSheetsClient::from_service_account()
                .await
                .map_err(|e| tf("errors.auth.sheetsConnectFailed", &[("error", &e.to_string())]))
        }
        Ok(None) => {
            if joined {
                Err(t("errors.auth.teamCodeNeedsServiceAccount"))
            } else {
                user_client().await
            }
        }
        Err(e) => {
            // Редкий I/O; для joined не уходим в user Google.
            if joined {
                Err(tf("errors.auth.saAtRestUnreadable", &[("error", &e)]))
            } else if service_account_cfg::path_exists() {
                Err(tf("errors.auth.saAtRestUnreadable", &[("error", &e)]))
            } else {
                user_client().await
            }
        }
    }
}

pub async fn find_user_by_credentials(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    login: &str,
    password: &str,
) -> Result<SheetUser, String> {
    let (user, _, needs_rehash) =
        find_user_by_credentials_scan(client, spreadsheet_id, login, password).await?;
    if needs_rehash {
        let _ = super::write::write_password(client, spreadsheet_id, user.row_index, password).await;
    }
    Ok(user)
}

/// Один GET A2:J1000: пользователь по login/password + Google email builtin Admin (если есть).
/// Третий элемент — нужно перезаписать plaintext → argon2 после успешного входа.
pub async fn find_user_by_credentials_scan(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    login: &str,
    password: &str,
) -> Result<(SheetUser, String, bool), String> {
    let range = format!("'{USERS_SHEET_NAME}'!A2:J1000");
    let data = client
        .read_values(spreadsheet_id, &range)
        .await
        .map_err(|e| tf("errors.auth.usersTableReadFailed", &[("error", &e.to_string())]))?;
    if data.is_empty() {
        return Err(t("errors.auth.usersTableEmpty"));
    }

    let login = login.trim();
    let mut found: Option<(SheetUser, bool)> = None;
    let mut admin_mail = String::new();
    let mut row_index = 2u32;
    for row in &data {
        let row_login = cell(row, 0);
        if row_login.is_empty() {
            row_index += 1;
            continue;
        }
        let parsed = parse_user(row, row_index);
        if admin_mail.is_empty() && crate::core::access::is_admin_role(&parsed.access_level) {
            admin_mail = parsed.google_email.trim().to_string();
        }
        if found.is_none() && row_login.eq_ignore_ascii_case(login) {
            let row_password = cell(row, 1);
            let (ok, needs_rehash) =
                crate::core::password_hash::verify_password(password, &row_password);
            if !ok {
                return Err(t("errors.auth.userNotFound"));
            }
            found = Some((parsed, needs_rehash));
        }
        row_index += 1;
    }
    let (user, needs_rehash) = found.ok_or_else(|| t("errors.auth.userNotFound"))?;
    Ok((user, admin_mail, needs_rehash))
}

pub async fn read_user_row(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
) -> Result<SheetUser, String> {
    if row_index < 2 {
        return Err(t("errors.auth.invalidUserRowIndex"));
    }
    let range = format!("'{USERS_SHEET_NAME}'!A{row_index}:J{row_index}");
    let data = client
        .read_values(spreadsheet_id, &range)
        .await
        .map_err(|e| tf("errors.auth.userRowReadFailed", &[("error", &e.to_string())]))?;
    let row = data
        .into_iter()
        .next()
        .ok_or_else(|| t("errors.auth.userRowEmpty"))?;
    if cell(&row, 0).is_empty() {
        return Err(t("errors.auth.userRowEmpty"));
    }
    Ok(parse_user(&row, row_index))
}

pub async fn list_users(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<Vec<SheetUser>, String> {
    let range = format!("'{USERS_SHEET_NAME}'!A2:J1000");
    let data = client
        .read_values(spreadsheet_id, &range)
        .await
        .map_err(|e| tf("errors.auth.usersReadFailed", &[("error", &e.to_string())]))?;
    let mut out = Vec::new();
    let mut row_index = 2u32;
    for row in data {
        if cell(&row, 0).is_empty() {
            row_index += 1;
            continue;
        }
        out.push(parse_user(&row, row_index));
        row_index += 1;
    }
    Ok(out)
}

pub async fn read_user_calc_config(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
) -> Result<String, String> {
    let range = format!("'{USERS_SHEET_NAME}'!I{row_index}");
    let rows = client.read_values(spreadsheet_id, &range).await?;
    Ok(rows
        .first()
        .and_then(|r| r.first())
        .map(|s| s.trim().to_string())
        .unwrap_or_default())
}
