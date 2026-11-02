//! Доступ к шаблонам: memberOk / HasOauth / Excel-only.

use crate::app_state::AppState;
use crate::auth::UserInfo;
use crate::core::access::{self, can_run_script, can_use_any_script_ex};
use crate::core::i18n::t;

/// Авторизованный пользователь сессии (общий helper для IPC).
pub fn require_user(state: &AppState) -> Result<UserInfo, String> {
    state
        .auth
        .current_user()
        .filter(|u| u.is_authorized)
        .ok_or_else(|| t("errors.auth.notAuthorized"))
}

/// Авторизованная сессия обязательна для IPC шаблонов.
pub fn require_session(state: &AppState) -> Result<(), String> {
    require_user(state).map(|_| ())
}

/// Admin или участник с HasOauth **и** локальным Google token.
pub fn session_has_oauth(state: &AppState) -> bool {
    state
        .auth
        .current_user()
        .map(|u| access::effective_has_oauth(&u.access_level, u.has_oauth))
        .unwrap_or(false)
}

/// Без своего OAuth не-админ пишет только в Excel.
pub fn templates_excel_only(state: &AppState) -> bool {
    !session_has_oauth(state)
}

fn session_level(state: &AppState) -> String {
    state
        .auth
        .current_user()
        .map(|u| u.access_level)
        .unwrap_or_else(|| "Denied".into())
}

/// Есть ли memberOk в пресете (общий helper для layout / templates).
pub fn any_member_ok_in_scope(state: &AppState, access_level: &str) -> bool {
    access::any_member_ok_in_scope(
        access_level,
        &|k| state.registry.is_member_ok(k),
        state
            .registry
            .list_with_meta()
            .into_iter()
            .any(|(_, _, m)| m),
    )
}

/// Хотя бы один разрешённый скрипт (вкладка «Шаблоны» / CRUD store).
pub fn require_any_script(state: &AppState) -> Result<(), String> {
    require_session(state)?;
    let level = session_level(state);
    if access::is_admin_role(&level) {
        return Ok(());
    }
    let has_oauth = session_has_oauth(state);
    let any_member_ok = any_member_ok_in_scope(state, &level);
    if can_use_any_script_ex(&level, has_oauth, any_member_ok) {
        Ok(())
    } else {
        Err(t("errors.templates.noScriptAccess"))
    }
}

pub fn require_script(state: &AppState, script_key: &str) -> Result<(), String> {
    require_session(state)?;
    let key = script_key.trim();
    if key.is_empty() {
        return require_any_script(state);
    }
    let level = session_level(state);
    let has_oauth = session_has_oauth(state);
    let member_ok = state.registry.is_member_ok(key);
    if can_run_script(&level, key, has_oauth, member_ok) {
        Ok(())
    } else {
        Err(access::deny_script_message(key))
    }
}

/// Запись в Google Sheets запрещена без HasOauth (кроме admin).
pub fn reject_google_write_if_locked(state: &AppState, export_to_excel: bool) -> Result<(), String> {
    if export_to_excel || session_has_oauth(state) {
        return Ok(());
    }
    Err(t("errors.templates.googleWriteNeedsOauth"))
}
