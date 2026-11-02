//! Компакт `templates.json` `CompactToJObjectByDefaults`:
//! на диск не пишем поля, равные serde-дефолтам. UI по-прежнему получает полный объект.

use crate::core::i18n::t;
use serde_json::{Map, Value};

/// Идентификаторы сущности шаблона/группы/триггера — не выкидывать даже пустыми.
/// `name` не здесь: в `subjects_filters` пустое `name` надо вырезать (дефолт "").
const KEEP_KEYS: &[&str] = &["id", "script_key"];

/// Ключи bool с дефолтом `true` (вырезаем `true`, оставляем `false`).
const BOOL_DEFAULT_TRUE: &[&str] = &[
    "selected",
    "cards_update_dimensions_mode",
    "is_enabled",
    "is_expanded",
];

/// Строковые дефолты десериализации (ключ → значение, которое можно опустить).
fn string_default(key: &str) -> Option<&'static str> {
    match key {
        "marketplace" => Some("wb"),
        "tab_name" | "selected_tab" | "default_tab" => None,
        "report_realization_v5_period" => Some("weekly"),
        "write_sort_direction" => Some("asc"),
        "locale" => Some("ru"),
        "type" | "trigger_type" => Some("Once"),
        _ => None,
    }
}

fn number_default(key: &str, n: &serde_json::Number) -> bool {
    match key {
        "limit" => n.as_i64() == Some(30) || n.as_f64() == Some(30.0),
        "offset" => n.as_i64() == Some(0) || n.as_f64() == Some(0.0),
        _ => false,
    }
}

fn is_default_tabs(arr: &[Value]) -> bool {
    let main = t("templates.defaultTab");
    let calc = t("templates.calculatorTab");
    matches!(
        arr,
        [Value::String(a), Value::String(b)] if a == &main && b == &calc
    )
}

fn is_default_tab_string(s: &str) -> bool {
    s == t("templates.defaultTab")
}

/// Вырезает дефолтные поля из дерева JSON перед записью на диск.
pub fn compact_value(mut v: Value) -> Value {
    compact_node(&mut v, None);
    v
}

/// `true` — родитель должен удалить этот узел.
fn compact_node(v: &mut Value, key: Option<&str>) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => {
            let k = key.unwrap_or("");
            if BOOL_DEFAULT_TRUE.contains(&k) {
                *b
            } else {
                !*b
            }
        }
        Value::String(s) => {
            if key.is_some_and(|k| KEEP_KEYS.contains(&k)) {
                return false;
            }
            if s.is_empty() {
                return true;
            }
            if let Some(k) = key {
                if matches!(k, "tab_name" | "selected_tab" | "default_tab") {
                    if is_default_tab_string(s) {
                        return true;
                    }
                } else if string_default(k) == Some(s.as_str()) {
                    return true;
                }
            }
            false
        }
        Value::Number(n) => key.is_some_and(|k| number_default(k, n)),
        Value::Array(arr) => {
            if let Some("tabs") = key {
                if is_default_tabs(arr) {
                    return true;
                }
            }
            let mut i = 0;
            while i < arr.len() {
                if compact_node(&mut arr[i], None) {
                    arr.remove(i);
                } else {
                    i += 1;
                }
            }
            arr.is_empty()
        }
        Value::Object(map) => {
            compact_object(map);
            map.is_empty()
        }
    }
}

fn compact_object(map: &mut Map<String, Value>) {
    let keys: Vec<String> = map.keys().cloned().collect();
    for k in keys {
        let Some(mut child) = map.remove(&k) else {
            continue;
        };
        if KEEP_KEYS.contains(&k.as_str()) {
            map.insert(k, child);
            continue;
        }
        if compact_node(&mut child, Some(k.as_str())) {
            continue;
        }
        map.insert(k, child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_empty_and_false() {
        let v = compact_value(json!({
            "id": "a",
            "name": "t",
            "script_key": "wb_cards",
            "spreadsheet_id": "",
            "clear_only_written_range": false,
            "cabinets": [],
            "marketplace": "wb",
            "tab_name": "Основная",
        }));
        assert_eq!(
            v,
            json!({
                "id": "a",
                "name": "t",
                "script_key": "wb_cards",
            })
        );
    }

    #[test]
    fn keeps_selected_false_strips_selected_true() {
        let v = compact_value(json!({
            "selected_fields": [
                {"name": "a", "title": "A", "selected": true},
                {"name": "b", "title": "B", "selected": false},
            ]
        }));
        assert_eq!(
            v,
            json!({
                "selected_fields": [
                    {"name": "a", "title": "A"},
                    {"name": "b", "title": "B", "selected": false},
                ]
            })
        );
    }

    #[test]
    fn strips_trigger_defaults() {
        let v = compact_value(json!({
            "time_triggers": [{
                "id": "t1",
                "type": "Once",
                "is_enabled": true,
                "template_id": "",
                "daily_time": null,
            }]
        }));
        assert_eq!(
            v,
            json!({
                "time_triggers": [{"id": "t1"}]
            })
        );
    }

    #[test]
    fn strips_subjects_filter_defaults() {
        let v = compact_value(json!({
            "subjects_filters": {
                "locale": "ru",
                "name": "",
                "limit": 30,
                "offset": 0,
            }
        }));
        assert!(v.as_object().map(|o| o.is_empty()).unwrap_or(false));
    }
}
