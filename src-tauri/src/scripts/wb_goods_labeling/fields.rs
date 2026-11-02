//! Поля `goods_labeling`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("amount", true),
        field("date", true),
        field("incomeId", true),
        field("shkId", true),
        field("sku", true),
        field("photoUrls", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.goods_labeling.{name}")),
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
    let report = root
        .get("report")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    for d in &report {
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
                    "nmId" => first_num(d, &["nmID", "nmId"]),
                    "amount" => first_num(d, &["amount"]),
                    "date" => fmt_ts(first_str(d, &["date"])),
                    "incomeId" => first_num(d, &["incomeId", "income_id"]),
                    "shkId" => first_num(d, &["shkID", "shkId"]),
                    "sku" => first_str(d, &["sku"]),
                    "photoUrls" => photos.clone(),
                    _ => String::new(),
                })
                .collect(),
        );
    }
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
