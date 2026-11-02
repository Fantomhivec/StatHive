//! Лист «Пользователи»: модель строки, парсинг, timestamps.

use chrono::{Local, NaiveDateTime, TimeZone, Utc};

pub const USERS_SHEET_NAME: &str = "Пользователи";

/// A login | B password | C display_name | D access_level | E google_email
/// | F session_id | G last_activity | H last_login | I calc_config | J has_oauth
pub(crate) const AUTH_TS: &str = "%Y-%m-%d %H:%M:%S";

pub const HEADER_ROW: &[&str] = &[
    "Login",
    "Password",
    "DisplayName",
    "AccessLevel",
    "GoogleEmail",
    "SessionId",
    "LastActivity",
    "LastLogin",
    "CalcConfig",
    "HasOauth",
];

#[derive(Debug, Clone)]
pub struct SheetUser {
    pub login: String,
    pub row_index: u32,
    pub display_name: String,
    pub access_level: String,
    pub google_email: String,
    pub last_login_utc: Option<chrono::DateTime<Utc>>,
    pub session_id: Option<String>,
    pub last_activity_utc: Option<chrono::DateTime<Utc>>,
    /// Id конфигурации калькулятора (лист CalcTeam). Пусто → default каталога.
    pub calc_config_id: String,
    /// Пользователь подключил свой Google OAuth (колонка J).
    pub has_oauth: bool,
}

impl SheetUser {
    pub fn display_name(&self) -> String {
        let d = self.display_name.trim();
        if d.is_empty() {
            self.login.clone()
        } else {
            d.to_string()
        }
    }

    pub fn last_login_display(&self) -> Option<String> {
        self.last_login_utc.map(|utc| {
            crate::core::sheet_dates::format_datetime(utc.with_timezone(&Local).naive_local())
        })
    }
}

pub(crate) fn parse_user(row: &[String], row_index: u32) -> SheetUser {
    let session_raw = cell(row, 5);
    SheetUser {
        login: cell(row, 0),
        row_index,
        display_name: cell(row, 2),
        access_level: normalize_role(&cell(row, 3)),
        google_email: cell(row, 4),
        session_id: if session_raw.is_empty() {
            None
        } else {
            Some(session_raw)
        },
        last_activity_utc: parse_auth_ts(&cell(row, 6)),
        last_login_utc: parse_auth_ts(&cell(row, 7)),
        calc_config_id: cell(row, 8),
        has_oauth: parse_has_oauth_cell(&cell(row, 9)),
    }
}

pub(crate) fn cell(row: &[String], idx: usize) -> String {
    row.get(idx)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn normalize_role(raw: &str) -> String {
    crate::core::access::role_storage_label_from_raw(raw)
}

pub(crate) fn format_auth_local(utc: chrono::DateTime<Utc>) -> String {
    utc.with_timezone(&Local).format(AUTH_TS).to_string()
}

fn parse_has_oauth_cell(raw: &str) -> bool {
    matches!(
        raw.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "да" | "y"
    )
}

pub fn parse_auth_ts(value: &str) -> Option<chrono::DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = normalize_auth_ts(trimmed);
    for fmt in [
        AUTH_TS,
        "%Y-%m-%d %H:%M",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, fmt) {
            return Local
                .from_local_datetime(&naive)
                .single()
                .map(|dt| dt.with_timezone(&Utc));
        }
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    None
}

fn normalize_auth_ts(raw: &str) -> String {
    let raw = raw.trim();
    let Some((date, time)) = raw.split_once(' ') else {
        return raw.to_string();
    };
    let mut parts = time.split(':');
    let Some(h) = parts.next() else {
        return raw.to_string();
    };
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return raw.to_string();
    }
    let h_pad = if h.len() == 1 {
        format!("0{h}")
    } else {
        h.to_string()
    };
    format!("{date} {h_pad}:{}", rest.join(":"))
}

/// Активная сессия на другом ПК.
pub fn has_active_foreign_session(user: &SheetUser, machine_id: &str) -> bool {
    let Some(session_id) = user.session_id.as_deref() else {
        return false;
    };
    if session_id.trim().is_empty() {
        return false;
    }
    let stored_machine = session_id.rsplit('_').next().unwrap_or("");
    if !stored_machine.is_empty() && stored_machine.eq_ignore_ascii_case(machine_id) {
        return false;
    }
    let Some(last) = user.last_activity_utc else {
        return false;
    };
    let age = Utc::now().signed_duration_since(last);
    age.num_minutes() <= 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auth_ts_accepts_single_digit_hour() {
        let dt = parse_auth_ts("2026-10-26 8:58:39").expect("single hour");
        let local = dt.with_timezone(&Local).naive_local();
        assert_eq!(
            local.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-10-26 08:58:39"
        );
    }
}
