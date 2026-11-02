//! Запись листа «Пользователи».

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::{t, tf};

use super::model::{format_auth_local, HEADER_ROW, USERS_SHEET_NAME};
use super::read::list_users;

pub async fn append_user_row(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    login: &str,
    password: &str,
    display_name: &str,
    access_level: &str,
    google_email: &str,
    calc_config_id: &str,
) -> Result<u32, String> {
    let users = list_users(client, spreadsheet_id).await?;
    for u in &users {
        if u.login.eq_ignore_ascii_case(login.trim()) {
            return Err(tf("errors.auth.loginTaken", &[("login", login.trim())]));
        }
    }
    let next_row = users
        .iter()
        .map(|u| u.row_index)
        .max()
        .unwrap_or(1)
        .saturating_add(1)
        .max(2);
    let hashed = crate::core::password_hash::hash_password(password)?;
    let range = format!("'{USERS_SHEET_NAME}'!A{next_row}:J{next_row}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![
                login.trim().to_string(),
                hashed,
                display_name.trim().to_string(),
                access_level.trim().to_string(),
                google_email.trim().to_string(),
                String::new(),
                String::new(),
                String::new(),
                calc_config_id.trim().to_string(),
                String::new(),
            ]],
        )
        .await?;
    Ok(next_row)
}

pub async fn write_header(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!A1:J1");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![HEADER_ROW.iter().map(|s| (*s).to_string()).collect()],
        )
        .await
}

pub async fn write_last_login(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    when: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!H{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![format_auth_local(when)]],
        )
        .await
}

pub async fn write_session(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    session_id: &str,
    activity: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    let sid_range = format!("'{USERS_SHEET_NAME}'!F{row_index}");
    let act_range = format!("'{USERS_SHEET_NAME}'!G{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &sid_range,
            vec![vec![session_id.to_string()]],
        )
        .await?;
    client
        .write_values(
            spreadsheet_id,
            &act_range,
            vec![vec![format_auth_local(activity)]],
        )
        .await
}

pub async fn clear_session_cells(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
) -> Result<(), String> {
    let sid_range = format!("'{USERS_SHEET_NAME}'!F{row_index}");
    let act_range = format!("'{USERS_SHEET_NAME}'!G{row_index}");
    client
        .write_values(spreadsheet_id, &sid_range, vec![vec![String::new()]])
        .await?;
    client
        .write_values(spreadsheet_id, &act_range, vec![vec![String::new()]])
        .await
}

pub async fn write_activity(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    when: chrono::DateTime<chrono::Utc>,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!G{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![format_auth_local(when)]],
        )
        .await
}

pub async fn write_password(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    new_password: &str,
) -> Result<(), String> {
    let hashed = crate::core::password_hash::hash_password(new_password)?;
    let range = format!("'{USERS_SHEET_NAME}'!B{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![hashed]],
        )
        .await
}

pub async fn write_access_level(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    role_id: &str,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!D{row_index}");
    let label = crate::core::access::role_storage_label_from_raw(role_id);
    client
        .write_values(spreadsheet_id, &range, vec![vec![label]])
        .await
}

pub async fn write_calc_config(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    calc_config_id: &str,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!I{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![calc_config_id.trim().to_string()]],
        )
        .await
}

pub async fn write_has_oauth(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    has: bool,
) -> Result<(), String> {
    let range = format!("'{USERS_SHEET_NAME}'!J{row_index}");
    let val = if has { "да" } else { "" };
    client
        .write_values(spreadsheet_id, &range, vec![vec![val.to_string()]])
        .await
}

pub async fn write_google_email(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
    email: &str,
) -> Result<(), String> {
    if row_index < 2 {
        return Err(t("errors.auth.invalidUserRowIndex"));
    }
    let range = format!("'{USERS_SHEET_NAME}'!E{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![email.trim().to_string()]],
        )
        .await
}

/// Удалить строку пользователя (только данные; Drive revoke — снаружи).
pub async fn delete_user_row(
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    row_index: u32,
) -> Result<(), String> {
    if row_index < 2 {
        return Err(t("errors.auth.invalidUserRowIndex"));
    }
    let range = format!("'{USERS_SHEET_NAME}'!A{row_index}:J{row_index}");
    client
        .write_values(
            spreadsheet_id,
            &range,
            vec![vec![
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]],
        )
        .await
}
