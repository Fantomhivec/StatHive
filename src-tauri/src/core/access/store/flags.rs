//! Лист «Пресеты» в workspace spreadsheet — единственный каталог прав (кроме builtin Admin).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;


pub const PRESETS_SHEET: &str = "Пресеты";
pub const ADMIN_ROLE_ID: &str = "Admin";

/// Права вкладок калькулятора + Excel.
pub const CALC_FLAG_VIEW: &str = "view";
pub const CALC_FLAG_SETTINGS: &str = "settings";
pub const CALC_FLAG_TABLES: &str = "tables";
pub const CALC_FLAG_KGT: &str = "kgt";
pub const CALC_FLAG_EXCEL: &str = "excel";

pub const ALL_CALC_FLAGS: &[&str] = &[
    CALC_FLAG_VIEW,
    CALC_FLAG_SETTINGS,
    CALC_FLAG_TABLES,
    CALC_FLAG_KGT,
    CALC_FLAG_EXCEL,
];

/// Вкладки приложения вне калькулятора.
pub const APP_FLAG_KEYS_CHECK: &str = "keysCheck";
pub const APP_FLAG_USERS: &str = "users";

pub const ALL_APP_FLAGS: &[&str] = &[APP_FLAG_KEYS_CHECK, APP_FLAG_USERS];

pub(crate) const HEADER: &[&str] =
    &["Id", "Title", "Calculator", "Scripts", "CalcAccess", "AppAccess"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessMatrix {
    All,
    Keys(HashSet<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDef {
    pub id: String,
    pub title: String,
    /// Есть хотя бы одно право калькулятора (кроме чистого excel).
    pub calculator: bool,
    pub scripts: AccessMatrix,
    /// Ключи: view, settings, tables, kgt, excel. Пусто + calculator=true → все.
    #[serde(default)]
    pub calc_access: HashSet<String>,
    /// Ключи: keysCheck, users. Пусто → нет вкладок.
    #[serde(default)]
    pub app_access: HashSet<String>,
}

pub(crate) fn cell(row: &[String], idx: usize) -> String {
    row.get(idx)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

pub(crate) fn calc_from_cell(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "1" | "true" | "yes" | "да" | "y"
    )
}

pub(crate) fn calc_cell(v: bool) -> String {
    if v { "1" } else { "0" }.into()
}

pub(crate) fn normalize_calc_flag(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "view" | "calc" | "calculator" => Some(CALC_FLAG_VIEW),
        "settings" | "setting" | "config" => Some(CALC_FLAG_SETTINGS),
        "tables" | "table" => Some(CALC_FLAG_TABLES),
        "kgt" => Some(CALC_FLAG_KGT),
        "excel" | "export" => Some(CALC_FLAG_EXCEL),
        _ => None,
    }
}

/// Парсинг CalcAccess. Пустая ячейка + Calculator=1 → все флаги.
pub fn parse_calc_access_cell(raw: &str, calculator_legacy: bool) -> HashSet<String> {
    let t = raw.trim();
    if t.is_empty() {
        if calculator_legacy {
            return ALL_CALC_FLAGS.iter().map(|s| (*s).to_string()).collect();
        }
        return HashSet::new();
    }
    if t == "*" {
        return ALL_CALC_FLAGS.iter().map(|s| (*s).to_string()).collect();
    }
    let mut set = HashSet::new();
    for part in t.split([',', ';', '\n', '|']) {
        if let Some(f) = normalize_calc_flag(part) {
            set.insert(f.to_string());
        }
    }
    set
}

pub fn calc_access_cell(flags: &HashSet<String>) -> String {
    if flags.is_empty() {
        return String::new();
    }
    if ALL_CALC_FLAGS.iter().all(|f| flags.contains(*f)) {
        return "*".into();
    }
    let mut v: Vec<&str> = ALL_CALC_FLAGS
        .iter()
        .copied()
        .filter(|f| flags.contains(*f))
        .collect();
    v.sort();
    v.join(",")
}

pub(crate) fn normalize_app_flag(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "keyscheck" | "keys_check" | "keys" => Some(APP_FLAG_KEYS_CHECK),
        "users" | "user" | "team" => Some(APP_FLAG_USERS),
        _ => None,
    }
}

/// Парсинг AppAccess. Пустая ячейка → нет вкладок; `*` → все.
pub fn parse_app_access_cell(raw: &str) -> HashSet<String> {
    let t = raw.trim();
    if t.is_empty() {
        return HashSet::new();
    }
    if t == "*" {
        return ALL_APP_FLAGS.iter().map(|s| (*s).to_string()).collect();
    }
    let mut set = HashSet::new();
    for part in t.split([',', ';', '\n', '|']) {
        if let Some(f) = normalize_app_flag(part) {
            set.insert(f.to_string());
        }
    }
    set
}

pub fn app_access_cell(flags: &HashSet<String>) -> String {
    if flags.is_empty() {
        return String::new();
    }
    if ALL_APP_FLAGS.iter().all(|f| flags.contains(*f)) {
        return "*".into();
    }
    let mut v: Vec<&str> = ALL_APP_FLAGS
        .iter()
        .copied()
        .filter(|f| flags.contains(*f))
        .collect();
    v.sort();
    v.join(",")
}

/// `calculator` bool = есть view|settings|tables|kgt (excel alone не включает режим калькулятора).
pub fn calculator_bool_from_flags(flags: &HashSet<String>) -> bool {
    flags.contains(CALC_FLAG_VIEW)
        || flags.contains(CALC_FLAG_SETTINGS)
        || flags.contains(CALC_FLAG_TABLES)
        || flags.contains(CALC_FLAG_KGT)
}

/// `*` = все скрипты; пусто = ни одного; иначе ключи через запятую.
pub fn parse_scripts_cell(raw: &str) -> AccessMatrix {
    let t = raw.trim();
    if t == "*" {
        return AccessMatrix::All;
    }
    if t.is_empty() {
        return AccessMatrix::Keys(HashSet::new());
    }
    let mut set = HashSet::new();
    for part in t.split([',', ';', '\n']) {
        let k = part.trim();
        if !k.is_empty() && k != "*" {
            set.insert(k.to_string());
        }
    }
    AccessMatrix::Keys(set)
}

pub fn scripts_cell(matrix: &AccessMatrix) -> String {
    match matrix {
        AccessMatrix::All => "*".into(),
        AccessMatrix::Keys(set) => {
            let mut v: Vec<_> = set.iter().cloned().collect();
            v.sort();
            v.join(",")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_access_parse_empty_star() {
        assert!(parse_app_access_cell("").is_empty());
        let all = parse_app_access_cell("*");
        assert!(all.contains(APP_FLAG_KEYS_CHECK));
        assert!(all.contains(APP_FLAG_USERS));
        let one = parse_app_access_cell("users,keysCheck");
        assert_eq!(one.len(), 2);
    }
}

