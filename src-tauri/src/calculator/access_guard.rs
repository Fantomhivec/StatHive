//! Проверки CalcAccess для IPC калькулятора.

use crate::app_state::AppState;
use crate::core::access::{
    self, deny_calc_message, CALC_FLAG_EXCEL, CALC_FLAG_KGT, CALC_FLAG_SETTINGS, CALC_FLAG_TABLES,
    CALC_FLAG_VIEW,
};
use serde_json::Value;

fn access_level(state: &AppState) -> Result<String, String> {
    Ok(crate::templates::access_gate::require_user(state)?.access_level)
}

pub fn require_flag(state: &AppState, flag: &str) -> Result<(), String> {
    let level = access_level(state)?;
    if access::can_calc(&level, flag) {
        Ok(())
    } else {
        Err(deny_calc_message(flag))
    }
}

pub fn require_view(state: &AppState) -> Result<(), String> {
    require_flag(state, CALC_FLAG_VIEW)
}

pub fn require_settings(state: &AppState) -> Result<(), String> {
    require_flag(state, CALC_FLAG_SETTINGS)
}

pub fn require_tables(state: &AppState) -> Result<(), String> {
    require_flag(state, CALC_FLAG_TABLES)
}

/// Refresh / URL / ручные поля / тип заполнения: admin или (tables + свой OAuth + token).
pub fn require_tables_mutate(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if access::is_admin_role(&u.access_level) {
        return Ok(());
    }
    if access::can_calc(&u.access_level, CALC_FLAG_TABLES)
        && access::effective_has_oauth(&u.access_level, u.has_oauth)
    {
        return Ok(());
    }
    Err(deny_calc_message(CALC_FLAG_TABLES))
}

pub fn require_kgt(state: &AppState) -> Result<(), String> {
    require_flag(state, CALC_FLAG_KGT)
}

pub fn require_excel(state: &AppState) -> Result<(), String> {
    require_flag(state, CALC_FLAG_EXCEL)
}

/// Любое право вкладки калькулятора (snapshot / warm).
pub fn require_any_calc_tab(state: &AppState) -> Result<(), String> {
    let level = access_level(state)?;
    if access::can_calculator_mode(&level) {
        Ok(())
    } else {
        Err(deny_calc_message(CALC_FLAG_VIEW))
    }
}

fn key_norm(k: &str) -> String {
    k.chars()
        .filter(|c| *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn is_kgt_key(n: &str) -> bool {
    n.contains("kgt")
}

fn is_tables_key(n: &str) -> bool {
    matches!(
        n,
        "tablemodes"
            | "tablelinks"
            | "columnbindings"
            | "lastupdated"
            | "templateidsbytablekey"
            | "datasourcemode"
    )
}

/// save_settings: флаги по ключам патча (КГТ / таблицы / остальное → settings).
pub fn require_save_settings(state: &AppState, settings: &Value) -> Result<(), String> {
    let Some(obj) = settings.as_object() else {
        return require_settings(state);
    };
    if obj.is_empty() {
        return require_settings(state);
    }
    let mut need_settings = false;
    let mut need_tables = false;
    let mut need_kgt = false;
    for k in obj.keys() {
        let n = key_norm(k);
        if is_kgt_key(&n) {
            need_kgt = true;
        } else if is_tables_key(&n) {
            need_tables = true;
        } else {
            need_settings = true;
        }
    }
    if need_kgt {
        require_kgt(state)?;
    }
    if need_tables {
        require_tables_mutate(state)?;
    }
    if need_settings {
        require_settings(state)?;
    }
    Ok(())
}
