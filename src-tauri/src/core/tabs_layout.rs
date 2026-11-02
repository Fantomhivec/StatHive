use crate::core::access::{
    self, CALC_FLAG_EXCEL, CALC_FLAG_KGT, CALC_FLAG_SETTINGS, CALC_FLAG_TABLES, CALC_FLAG_VIEW,
};
use crate::core::config::AppConfig;
use crate::core::i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabLayoutItem {
    pub tab_key: String,
    #[serde(default = "default_true")]
    pub is_visible: bool,
    #[serde(default)]
    pub order: i32,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TabsFile {
    #[serde(default)]
    layouts: HashMap<String, Vec<TabLayoutItem>>,
    #[serde(default)]
    calculator_mode: HashMap<String, bool>,
}

fn path() -> PathBuf {
    AppConfig::config_dir().join("tabs.layout.json")
}

fn load_file() -> TabsFile {
    let p = path();
    if !p.is_file() {
        return TabsFile::default();
    }
    fs::read_to_string(&p)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_file(file: &TabsFile) -> Result<(), String> {
    let dir = AppConfig::config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    fs::write(path(), raw).map_err(|e| e.to_string())
}

/// Вкладки обычного режима (без калькуляторных — «Таблицы» только в CALCULATOR_NAV).
pub fn default_tabs() -> Vec<(String, String)> {
    vec![
        ("KeysCheck".into(), t("tabs.KeysCheck")),
        ("Templates".into(), t("tabs.Templates")),
        ("Logs".into(), t("tabs.Logs")),
    ]
}

/// Admin + User (см. `core::access`).
pub fn can_calculator_mode(access_level: &str) -> bool {
    crate::core::access::can_calculator_mode(access_level)
}

fn calc_flags_json(access_level: &str) -> serde_json::Value {
    serde_json::json!({
        "view": access::can_calc(access_level, CALC_FLAG_VIEW),
        "settings": access::can_calc(access_level, CALC_FLAG_SETTINGS),
        "tables": access::can_calc(access_level, CALC_FLAG_TABLES),
        "kgt": access::can_calc(access_level, CALC_FLAG_KGT),
        "excel": access::can_calc(access_level, CALC_FLAG_EXCEL),
    })
}

pub fn get_layout(
    login: &str,
    access_level: &str,
    has_oauth: bool,
    any_member_ok_in_scope: bool,
    can_check_google: bool,
) -> serde_json::Value {
    let login = login.trim();
    let file = load_file();
    let saved = file.layouts.get(login).cloned().unwrap_or_default();
    let by_key: HashMap<String, TabLayoutItem> = saved
        .into_iter()
        .map(|x| (x.tab_key.to_ascii_lowercase(), x))
        .collect();

    let mut items = Vec::new();
    for (idx, (key, title)) in default_tabs().into_iter().enumerate() {
        let saved = by_key.get(&key.to_ascii_lowercase());
        items.push(serde_json::json!({
            "tab_key": key,
            "tabKey": key,
            "title": title,
            "is_visible": saved.map(|s| s.is_visible).unwrap_or(true),
            "isVisible": saved.map(|s| s.is_visible).unwrap_or(true),
            "order": saved.map(|s| s.order).unwrap_or(idx as i32),
        }));
    }
    items.sort_by_key(|v| v.get("order").and_then(|o| o.as_i64()).unwrap_or(0));

    let can = can_calculator_mode(access_level);
    let has_env_keys = crate::core::env_check::has_usable_key_configured();
    let can_templates = has_env_keys
        && access::can_use_any_script_ex(access_level, has_oauth, any_member_ok_in_scope);
    // Логи / Таблицы (calc) — только при действующем ключе в .env.
    let can_logs = has_env_keys;
    let can_tables_tab =
        has_env_keys && access::can_calc(access_level, CALC_FLAG_TABLES);
    let force =
        access::force_calculator_shell(access_level, has_oauth, any_member_ok_in_scope);
    let stored = file
        .calculator_mode
        .get(login)
        .copied()
        .unwrap_or(false);
    let calculator_mode = if force { can } else { can && stored };
    let flags = calc_flags_json(access_level);
    let is_admin = access::is_admin_role(access_level);
    let effective_oauth = is_admin || has_oauth;

    serde_json::json!({
        "login": login,
        "items": items,
        "calculator_mode": calculator_mode,
        "calculatorMode": calculator_mode,
        "can_calculator_mode": can,
        "canCalculatorMode": can,
        "force_calculator_mode": force,
        "forceCalculatorMode": force,
        "can_templates": can_templates,
        "canTemplates": can_templates,
        "can_logs": can_logs,
        "canLogs": can_logs,
        "has_env_keys": has_env_keys,
        "hasEnvKeys": has_env_keys,
        "can_tables_tab": can_tables_tab,
        "canTablesTab": can_tables_tab,
        "calc_access": flags,
        "calcAccess": flags,
        "can_keys_check": access::can_keys_check(access_level),
        "canKeysCheck": access::can_keys_check(access_level),
        "can_users_tab": access::can_users_tab(access_level),
        "canUsersTab": access::can_users_tab(access_level),
        "has_oauth": effective_oauth,
        "hasOauth": effective_oauth,
        "templates_excel_only": !effective_oauth,
        "templatesExcelOnly": !effective_oauth,
        "tables_can_mutate": is_admin || (effective_oauth && access::can_calc(access_level, CALC_FLAG_TABLES)),
        "tablesCanMutate": is_admin || (effective_oauth && access::can_calc(access_level, CALC_FLAG_TABLES)),
        "is_main_admin": is_admin,
        "isMainAdmin": is_admin,
        "can_check_google": can_check_google,
        "canCheckGoogle": can_check_google,
    })
}

pub fn save_layout(
    login: &str,
    access_level: &str,
    items: Vec<TabLayoutItem>,
    calculator_mode: Option<bool>,
    has_oauth: bool,
    any_member_ok_in_scope: bool,
) -> Result<serde_json::Value, String> {
    let login = login.trim().to_string();
    if login.is_empty() {
        return Err(t("errors.tabs.noUser"));
    }
    let mut file = load_file();
    // Не сохраняем калькуляторные ключи в раскладку обычного режима.
    let allowed: HashMap<String, ()> = default_tabs()
        .into_iter()
        .map(|(k, _)| (k.to_ascii_lowercase(), ()))
        .collect();
    let mut list: Vec<TabLayoutItem> = items
        .into_iter()
        .filter(|it| allowed.contains_key(&it.tab_key.to_ascii_lowercase()))
        .collect();
    if !list.is_empty() && list.iter().all(|x| !x.is_visible) {
        list[0].is_visible = true;
    }
    for (idx, it) in list.iter_mut().enumerate() {
        if it.order == 0 {
            it.order = idx as i32;
        }
    }
    file.layouts.insert(login.clone(), list);

    let force =
        access::force_calculator_shell(access_level, has_oauth, any_member_ok_in_scope);
    if let Some(want) = calculator_mode {
        let can = can_calculator_mode(access_level);
        if want && !can {
            return Err(t("errors.tabs.calculatorModeDenied"));
        }
        // Принудительный режим: нельзя выключить.
        let stored = if force { true } else { want && can };
        file.calculator_mode.insert(login.clone(), stored);
    } else if force {
        file.calculator_mode.insert(login.clone(), true);
    }

    save_file(&file)?;
    let can_check = crate::core::oauth_config::can_check_google(access_level, &login);
    Ok(get_layout(
        &login,
        access_level,
        has_oauth,
        any_member_ok_in_scope,
        can_check,
    ))
}
