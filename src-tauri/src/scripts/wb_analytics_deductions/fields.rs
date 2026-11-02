//! Поля `analytics_deductions` (подмены / неверные вложения).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("dtBonus", true),
        field("nmId", true),
        field("bonusSumm", true),
        field("bonusType", true),
        field("oldVendorCode", true),
        field("newVendorCode", true),
        field("oldSku", true),
        field("newSku", true),
        field("oldSize", true),
        field("newSize", true),
        field("photoUrls", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.analytics_deductions.{name}")),
        selected,
    }
}

pub fn append_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let data = root.get("data").unwrap_or(root);
    let reports = data
        .get("reports")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    for d in &reports {
        let photos = d
            .get("photoUrls")
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        out.push(
            selected
                .iter()
                .map(|f| match f.name.as_str() {
                    "cabinet" => cabinet.to_string(),
                    "dtBonus" => fmt_ts(first_str(d, &["dtBonus", "dt_bonus"])),
                    "nmId" => first_num(d, &["nmId", "nmID"]),
                    "bonusSumm" => first_num(d, &["bonusSumm", "bonus_summ"]),
                    "bonusType" => first_str(d, &["bonusType", "bonus_type"]),
                    "oldVendorCode" => first_str(d, &["oldVendorCode"]),
                    "newVendorCode" => first_str(d, &["newVendorCode"]),
                    "oldSku" => first_str(d, &["oldSku"]),
                    "newSku" => first_str(d, &["newSku"]),
                    "oldSize" => first_str(d, &["oldSize"]),
                    "newSize" => first_str(d, &["newSize"]),
                    "photoUrls" => photos.clone(),
                    _ => String::new(),
                })
                .collect(),
        );
    }
}

pub fn total_count(root: &Value) -> Option<i64> {
    root.get("data")
        .and_then(|d| d.get("total"))
        .and_then(|t| t.as_i64())
        .or_else(|| root.get("total").and_then(|t| t.as_i64()))
}

fn fmt_ts(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    sheet_dates::format_wb_timestamp(t)
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(Value::String(s)) = v.get(*k) {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    String::new()
}

fn first_num(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match v.get(*k) {
            Some(Value::Number(n)) => return n.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() => return s.clone(),
            _ => {}
        }
    }
    String::new()
}
