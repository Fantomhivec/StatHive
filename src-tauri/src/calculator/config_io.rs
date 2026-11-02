//! Экспорт / импорт пакета конфигурации калькулятора (паритет).

use crate::calculator::profile::{self, CalculatorProfile, OzKgtRules, WbKgtRules};
use crate::calculator::snapshot;
use crate::core::access::{self, CALC_FLAG_KGT, CALC_FLAG_SETTINGS, CALC_FLAG_TABLES};
use chrono::Local;
use serde_json::{json, Map, Value};
use std::fs;

fn strip_profile_for_access(mut p: CalculatorProfile, level: Option<&str>) -> CalculatorProfile {
    let Some(level) = level.filter(|s| !s.is_empty()) else {
        return p;
    };
    if access::is_admin_role(level) {
        return p;
    }
    if !access::can_calc(level, CALC_FLAG_TABLES) {
        p.table_links.clear();
        p.table_modes.clear();
        p.column_bindings.clear();
        p.last_updated.clear();
        p.template_ids_by_table_key.clear();
    }
    if !access::can_calc(level, CALC_FLAG_SETTINGS) {
        p.tax_percent = 0.0;
        p.acquiring_percent = 0.0;
        p.adv_percent = 0.0;
        p.extra_expenses_percent = 0.0;
        p.ozon_tax_percent = 0.0;
        p.ozon_acquiring_percent = 0.0;
        p.ozon_adv_percent = 0.0;
        p.ozon_extra_expenses_percent = 0.0;
        p.profit_formula.clear();
        p.ozon_profit_formula.clear();
        p.triggers_enabled = false;
    }
    if !access::can_calc(level, CALC_FLAG_KGT) {
        p.wb_kgt = WbKgtRules::default();
        p.oz_kgt = OzKgtRules::default();
    }
    p
}

pub fn build_config_json_for_access(level: Option<&str>) -> Result<String, String> {
    // Прогреть ensure_* через snapshot
    let _ = snapshot::get_snapshot();
    let profile = strip_profile_for_access(profile::load(), level);
    let exported_at = Local::now().format("%d.%m.%Y %H:%M:%S").to_string();

    let packet = json!({
        "kind": "universal-calculator-config",
        "version": 4,
        "exportedAt": exported_at,
        "profile": profile,
        "templates": []
    });
    serde_json::to_string_pretty(&packet).map_err(|e| e.to_string())
}

pub fn export_config_file_for_access(path: &str, level: Option<&str>) -> Result<Value, String> {
    let path = path.trim();
    if path.is_empty() {
        return Ok(json!({ "ok": false, "cancelled": true }));
    }
    let json = build_config_json_for_access(level)?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(path, json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(json!({
        "ok": true,
        "cancelled": false,
        "path": path,
        "json": json
    }))
}

pub fn import_config(json_raw: &str) -> Result<Value, String> {
    let json = json_raw.trim();
    if json.is_empty() {
        return Err("Пустая конфигурация".into());
    }
    let root: Value = serde_json::from_str(json).map_err(|e| format!("JSON: {e}"))?;
    let kind = root
        .get("kind")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if !kind.eq_ignore_ascii_case("universal-calculator-config") {
        return Err("Это не пакет конфигурации калькулятора".into());
    }

    let profile_val = root
        .get("profile")
        .cloned()
        .ok_or_else(|| "Не удалось прочитать профиль".to_string())?;
    let profile_val = normalize_profile_json(profile_val);

    let mut imported: CalculatorProfile =
        serde_json::from_value(profile_val).map_err(|e| format!("Профиль: {e}"))?;

    imported.triggers_enabled = false;
    imported.triggers.clear();
    imported.bundle_id = uuid::Uuid::new_v4().simple().to_string();
    // templates из пакета: createOrUpdateAll заведёт шаблоны по tableLinks при необходимости
    imported.template_ids_by_table_key.clear();

    profile::save(&imported)?;
    Ok(snapshot::get_snapshot())
}

pub fn import_config_file(path: &str) -> Result<Value, String> {
    let path = path.trim();
    if path.is_empty() {
        return Ok(json!({ "ok": false, "cancelled": true }));
    }
    let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let snapshot = import_config(&json)?;
    Ok(json!({
        "ok": true,
        "cancelled": false,
        "path": path,
        "snapshot": snapshot
    }))
}

/// пишет PascalCase; UR — camelCase. Приводим ключи профиля к camelCase.
fn normalize_profile_json(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, val) in map {
                let key = pascal_to_camel(&k);
                let nested = if key == "wbKgt" || key == "ozKgt" || key == "tableModes"
                    || key == "tableLinks"
                    || key == "lastUpdated"
                    || key == "cabinetMarketplaces"
                    || key == "templateIdsByTableKey"
                {
                    normalize_keys_deep(val)
                } else if key == "columnBindings" || key == "selectedCabinets" || key == "triggers"
                {
                    normalize_keys_deep(val)
                } else {
                    normalize_keys_deep(val)
                };
                out.insert(key, nested);
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn normalize_keys_deep(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, val) in map {
                out.insert(pascal_to_camel(&k), normalize_keys_deep(val));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_keys_deep).collect()),
        other => other,
    }
}

fn pascal_to_camel(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    // Already camelCase / lowercase start
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if first.is_lowercase() {
        return s.to_string();
    }
    // Acronyms like WbKgt → wbKgt; OzKgt → ozKgt
    let rest: String = chars.collect();
    if rest.is_empty() {
        return first.to_lowercase().to_string();
    }
    // If second char is uppercase (WB…) keep first lower only if whole is acronym — simple: lower first
    format!("{}{}", first.to_lowercase(), rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_keys() {
        assert_eq!(pascal_to_camel("TaxPercent"), "taxPercent");
        assert_eq!(pascal_to_camel("WbKgt"), "wbKgt");
        assert_eq!(pascal_to_camel("taxPercent"), "taxPercent");
    }
}
