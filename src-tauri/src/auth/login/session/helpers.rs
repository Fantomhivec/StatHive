// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Session helpers shared by login / refresh / activity.
use super::super::super::{UserInfo, sheets_users::SheetUser};
use crate::core::access;
use sha2::{Digest, Sha256};

pub(crate) fn user_from_sheet(found: &SheetUser, session_id: String) -> UserInfo {
    let has_oauth =
        access::is_admin_role(&found.access_level) || found.has_oauth;
    UserInfo {
        login: found.login.clone(),
        display_name: found.display_name(),
        is_authorized: true,
        access_level: found.access_level.clone(),
        access_level_title: access::access_level_title(&found.access_level),
        row_index: found.row_index,
        session_id: Some(session_id),
        birth_date: String::new(),
        last_login: found.last_login_display().unwrap_or_default(),
        google_email: found.google_email.clone(),
        has_oauth,
    }
}

pub(crate) fn sheet_access_revoked(access_level: &str) -> bool {
    !access::is_admin_role(access_level) && access::find_preset(access_level).is_none()
}

pub(crate) fn session_caps_changed(prev: &UserInfo, next: &UserInfo) -> bool {
    prev.access_level != next.access_level
        || prev.has_oauth != next.has_oauth
        || prev.google_email != next.google_email
        || prev.display_name != next.display_name
}

/// Снимок флагов пресета (AppAccess / CalcAccess / Scripts) — ловит правки без смены id.
pub(crate) fn preset_caps_fingerprint(access_level: &str) -> String {
    if access::is_admin_role(access_level) {
        return "admin".into();
    }
    let Some(p) = access::find_preset(access_level) else {
        return format!("none:{}", access_level.trim().to_ascii_lowercase());
    };
    let mut calc: Vec<_> = p.calc_access.iter().cloned().collect();
    calc.sort();
    let mut app: Vec<_> = p.app_access.iter().cloned().collect();
    app.sort();
    let scripts = match &p.scripts {
        access::AccessMatrix::All => "*".into(),
        access::AccessMatrix::Keys(set) => {
            let mut keys: Vec<_> = set.iter().cloned().collect();
            keys.sort();
            keys.join(",")
        }
    };
    format!(
        "{}|calc:{}|c:{}|a:{}|s:{}",
        p.id.to_ascii_lowercase(),
        if p.calculator { "1" } else { "0" },
        calc.join(","),
        app.join(","),
        scripts
    )
}

/// Login строки Users должен совпадать с сессией (защита от подмены row_index).
/// Пустой локальный SessionId — не ok (иначе kick/сверка обходятся).
pub(crate) fn sheet_row_matches_session(user: &UserInfo, row: &SheetUser) -> bool {
    let sheet_login = row.login.trim();
    if sheet_login.is_empty()
        || !sheet_login.eq_ignore_ascii_case(user.login.trim())
    {
        return false;
    }
    let local_sid = user
        .session_id
        .as_deref()
        .unwrap_or("")
        .trim();
    if local_sid.is_empty() {
        return false;
    }
    let sheet_sid = row.session_id.as_deref().unwrap_or("").trim();
    !sheet_sid.is_empty() && sheet_sid == local_sid
}

pub(crate) fn machine_id() -> String {
    let info = format!(
        "{}_{}_{}",
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "pc".into()),
        std::env::var("USERNAME").unwrap_or_else(|_| "user".into()),
        std::env::var("USERDOMAIN").unwrap_or_else(|_| "dom".into()),
    );
    let hash = Sha256::digest(info.as_bytes());
    hex::encode(&hash[..8])
}

/// Transport / timeout Sheets — не credential; UI без URL.
pub(crate) fn is_sheets_transport_error(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("error sending request for url")
        || e.contains("error trying to connect")
        || e.contains("timed out")
        || e.contains("connection reset")
        || e.contains("connection refused")
        || e.contains("dns error")
        || e.contains("failed to lookup")
}
