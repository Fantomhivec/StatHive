//! Access control: builtin Admin + пресеты прав из листа «Пресеты».

mod store;
mod ui_cache;

pub use store::{
    delete_preset, ensure_access_sheets, list_presets, reload_presets_cache, resolve_preset_id,
    save_preset, AccessMatrix, PresetDef, ADMIN_ROLE_ID, ALL_APP_FLAGS, ALL_CALC_FLAGS,
    APP_FLAG_KEYS_CHECK, APP_FLAG_USERS, CALC_FLAG_EXCEL, CALC_FLAG_KGT, CALC_FLAG_SETTINGS,
    CALC_FLAG_TABLES, CALC_FLAG_VIEW,
};
pub use ui_cache::{
    cached_team, invalidate_team_cache, invalidate_users_ui_cache, set_team_cache,
    users_ui_cache_ready, TeamCacheRow,
};

use std::sync::Mutex;

static PRESETS_CACHE: Mutex<Option<Vec<PresetDef>>> = Mutex::new(None);

/// Канонический builtin Admin (колонка AccessLevel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleId {
    Admin,
}

impl RoleId {
    pub fn as_str(self) -> &'static str {
        ADMIN_ROLE_ID
    }
}

pub fn set_presets_cache(presets: Vec<PresetDef>) {
    if let Ok(mut g) = PRESETS_CACHE.lock() {
        *g = Some(presets);
    }
}

pub(crate) fn clear_presets_cache_inner() {
    if let Ok(mut g) = PRESETS_CACHE.lock() {
        *g = None;
    }
}

/// Drop RAM presets (activity timer fail-closed / tests).
pub fn clear_presets_cache() {
    clear_presets_cache_inner();
}

pub fn presets_cache_ready() -> bool {
    PRESETS_CACHE
        .lock()
        .ok()
        .map(|g| g.is_some())
        .unwrap_or(false)
}

pub fn cached_presets() -> Vec<PresetDef> {
    PRESETS_CACHE
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default()
}

pub fn find_preset(access_level: &str) -> Option<PresetDef> {
    let id = access_level.trim();
    if id.is_empty() {
        return None;
    }
    cached_presets()
        .into_iter()
        .find(|p| p.id.eq_ignore_ascii_case(id))
}

/// Builtin Admin (или legacy «Админ»).
pub fn is_admin_role(raw: &str) -> bool {
    let t = raw.trim();
    if t.is_empty() {
        return false;
    }
    let compact = t
        .to_lowercase()
        .replace([' ', '_', '-', '\u{00a0}'], "");
    matches!(compact.as_str(), "admin" | "админ")
}

/// SA / код команды / удаление пользователей / каталог CalcTeam / открытие листа Users.
pub fn can_manage_team(access_level: &str) -> bool {
    is_admin_role(access_level)
}

fn has_app_flag(access_level: &str, flag: &str) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    let Some(preset) = find_preset(access_level) else {
        return false;
    };
    preset
        .app_access
        .iter()
        .any(|f| f.eq_ignore_ascii_case(flag))
}

/// Вкладка «Проверка ключей» (+ подключение своего OAuth).
pub fn can_keys_check(access_level: &str) -> bool {
    has_app_flag(access_level, APP_FLAG_KEYS_CHECK)
}

/// Вкладка «Пользователи» (инвайт, пресеты; без SA / delete / открыть лист).
pub fn can_users_tab(access_level: &str) -> bool {
    has_app_flag(access_level, APP_FLAG_USERS)
}

/// Для колонки AccessLevel: Admin как есть; иначе id пресета из кэша или raw.
pub fn role_storage_label_from_raw(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return "Denied".into();
    }
    if is_admin_role(t) {
        return ADMIN_ROLE_ID.into();
    }
    if let Some(p) = find_preset(t) {
        return p.id;
    }
    t.to_string()
}

/// Право на скрипт по пресету (без учёта member_ok / OAuth — см. `can_run_script`).
pub fn can_use_script(access_level: &str, script_key: &str) -> bool {
    let key = script_key.trim();
    if key.is_empty() {
        return false;
    }
    if is_admin_role(access_level) {
        return true;
    }
    match find_preset(access_level) {
        Some(preset) => match &preset.scripts {
            AccessMatrix::All => true,
            AccessMatrix::Keys(set) => set.contains(key),
        },
        None => false,
    }
}

/// Effective OAuth для caps/IPC: admin, либо (флаг HasOauth **и** локальный token).
pub fn effective_has_oauth(access_level: &str, has_oauth_flag: bool) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    has_oauth_flag && crate::core::google::oauth::has_cached_token()
}

/// Есть ли memberOk среди скриптов пресета (All → любой в реестре).
pub fn any_member_ok_in_scope(
    access_level: &str,
    is_member_ok: &dyn Fn(&str) -> bool,
    registry_has_any_member_ok: bool,
) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    let Some(preset) = find_preset(access_level) else {
        return false;
    };
    match &preset.scripts {
        AccessMatrix::All => registry_has_any_member_ok,
        AccessMatrix::Keys(set) => set.iter().any(|k| is_member_ok(k)),
    }
}

/// Запуск скрипта: пресет + (admin | has_oauth | member_ok).
pub fn can_run_script(
    access_level: &str,
    script_key: &str,
    has_oauth: bool,
    script_member_ok: bool,
) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    if !can_use_script(access_level, script_key) {
        return false;
    }
    has_oauth || script_member_ok
}

/// Есть ли право хотя бы на один скрипт (вкладка «Шаблоны»).
/// `any_member_ok` — среди ключей пресета (или всего реестра при All) есть member_ok.
pub fn can_use_any_script_ex(
    access_level: &str,
    has_oauth: bool,
    any_member_ok_in_scope: bool,
) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    match find_preset(access_level) {
        Some(preset) => match &preset.scripts {
            AccessMatrix::All => {
                if has_oauth {
                    true
                } else {
                    any_member_ok_in_scope
                }
            }
            AccessMatrix::Keys(set) => {
                if set.is_empty() {
                    return false;
                }
                if has_oauth {
                    return true;
                }
                any_member_ok_in_scope
            }
        },
        None => false,
    }
}

pub fn can_calculator_mode(access_level: &str) -> bool {
    can_calc(access_level, CALC_FLAG_VIEW)
        || can_calc(access_level, CALC_FLAG_SETTINGS)
        || can_calc(access_level, CALC_FLAG_TABLES)
        || can_calc(access_level, CALC_FLAG_KGT)
}

/// Право на флаг калькулятора (`view` / `settings` / `tables` / `kgt` / `excel`).
pub fn can_calc(access_level: &str, flag: &str) -> bool {
    if is_admin_role(access_level) {
        return true;
    }
    let flag = flag.trim().to_ascii_lowercase();
    if flag.is_empty() {
        return false;
    }
    let Some(preset) = find_preset(access_level) else {
        return false;
    };
    if preset.calc_access.is_empty() {
        // Legacy: только calculator bool без CalcAccess.
        return preset.calculator && ALL_CALC_FLAGS.iter().any(|f| f.eq_ignore_ascii_case(&flag));
    }
    preset
        .calc_access
        .iter()
        .any(|f| f.eq_ignore_ascii_case(&flag))
}

/// Нет доступных скриптов (с учётом OAuth/memberOk) + есть калькулятор →
/// принудительный режим калькулятора.
pub fn force_calculator_shell(
    access_level: &str,
    has_oauth: bool,
    any_member_ok_in_scope: bool,
) -> bool {
    can_calculator_mode(access_level)
        && !can_use_any_script_ex(access_level, has_oauth, any_member_ok_in_scope)
}

pub fn deny_script_message(script_key: &str) -> String {
    crate::core::i18n::tf(
        "errors.access.scriptDenied",
        &[("scriptKey", script_key)],
    )
}

pub fn deny_calc_message(flag: &str) -> String {
    crate::core::i18n::tf("errors.access.calcDenied", &[("flag", flag)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Mutex;

    /// Общий кэш пресетов — тесты не параллелить.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn admin_detect() {
        assert!(is_admin_role("Admin"));
        assert!(is_admin_role("Админ"));
        assert!(!is_admin_role("User"));
    }

    #[test]
    fn preset_matrix_and_calc_only() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_presets_cache(vec![PresetDef {
            id: "Analyst".into(),
            title: "Analyst".into(),
            calculator: false,
            scripts: AccessMatrix::Keys(HashSet::from(["cards_list".into()])),
            calc_access: HashSet::new(),
            app_access: HashSet::new(),
        }]);
        assert!(can_use_script("Analyst", "cards_list"));
        assert!(!can_use_script("Analyst", "prices_by_nm"));
        assert!(!can_calculator_mode("Analyst"));
        assert!(can_use_any_script_ex("Analyst", true, true));
        assert!(!can_keys_check("Analyst"));
        assert!(!can_users_tab("Analyst"));

        set_presets_cache(vec![PresetDef {
            id: "CalcOnly".into(),
            title: "Calc".into(),
            calculator: true,
            scripts: AccessMatrix::Keys(HashSet::new()),
            calc_access: HashSet::from([CALC_FLAG_VIEW.into()]),
            app_access: HashSet::from([APP_FLAG_USERS.into(), APP_FLAG_KEYS_CHECK.into()]),
        }]);
        assert!(can_calculator_mode("CalcOnly"));
        assert!(can_calc("CalcOnly", CALC_FLAG_VIEW));
        assert!(!can_calc("CalcOnly", CALC_FLAG_SETTINGS));
        assert!(force_calculator_shell("CalcOnly", true, true));
        assert!(!can_use_any_script_ex("CalcOnly", true, true));
        assert!(can_users_tab("CalcOnly"));
        assert!(can_keys_check("CalcOnly"));
        assert!(can_keys_check("Admin"));
        clear_presets_cache();
    }

    #[test]
    fn member_ok_gate_and_oauth_unlock() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_presets_cache(vec![PresetDef {
            id: "Member".into(),
            title: "Member".into(),
            calculator: false,
            scripts: AccessMatrix::Keys(HashSet::from([
                "cards_list".into(),
                "prices_by_nm".into(),
            ])),
            calc_access: HashSet::new(),
            app_access: HashSet::new(),
        }]);
        // Без OAuth: только member_ok скрипты.
        assert!(!can_run_script("Member", "cards_list", false, false));
        assert!(can_run_script("Member", "cards_list", false, true));
        assert!(!can_run_script("Member", "prices_by_nm", false, false));
        // С OAuth: весь пресет.
        assert!(can_run_script("Member", "cards_list", true, false));
        assert!(can_run_script("Member", "prices_by_nm", true, false));
        // Вне пресета — нет.
        assert!(!can_run_script("Member", "unknown", true, true));
        assert!(!can_use_any_script_ex("Member", false, false));
        assert!(can_use_any_script_ex("Member", false, true));
        assert!(can_use_any_script_ex("Member", true, false));
        clear_presets_cache();
    }
}
