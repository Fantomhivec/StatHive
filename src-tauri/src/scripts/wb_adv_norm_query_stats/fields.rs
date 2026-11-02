//! Поля `wb_adv_norm_query_stats`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("advertId", true),
        field("nmId", true),
        field("normQuery", true),
        field("date", true),
        field("views", true),
        field("clicks", true),
        field("ctr", true),
        field("cpc", true),
        field("sum", true),
        field("atbs", true),
        field("orders", true),
        field("cr", true),
        field("avgPosition", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_norm_query_stats.{name}")),
        selected,
    }
}

pub fn append_stats_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    root: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let items = root
        .get("stats")
        .or_else(|| root.get("items"))
        .or_else(|| root.as_array().map(|_| root))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            if root.is_object() {
                vec![root.clone()]
            } else {
                Vec::new()
            }
        });
    for item in &items {
        let advert_id = first_num(item, &["advert_id", "advertId", "advertID"]);
        let nm_id = first_num(item, &["nm_id", "nmId", "nmID"]);
        let norm_query = first_str(item, &["norm_query", "normQuery", "query"]);
        let days = item
            .get("days")
            .or_else(|| item.get("stats"))
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        if days.is_empty() {
            out.push(row(
                &selected,
                cabinet,
                &advert_id,
                &nm_id,
                &norm_query,
                &fmt_date(first_str(item, &["date", "dt"])),
                item,
            ));
            continue;
        }
        for day in &days {
            out.push(row(
                &selected,
                cabinet,
                &advert_id,
                &nm_id,
                &norm_query,
                &fmt_date(first_str(day, &["date", "dt"])),
                day,
            ));
        }
    }
}

fn row(
    selected: &[&ScriptField],
    cabinet: &str,
    advert_id: &str,
    nm_id: &str,
    norm_query: &str,
    date: &str,
    m: &Value,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "advertId" => advert_id.to_string(),
            "nmId" => nm_id.to_string(),
            "normQuery" => norm_query.to_string(),
            "date" => date.to_string(),
            "views" => first_num(m, &["views", "views_count"]),
            "clicks" => first_num(m, &["clicks"]),
            "ctr" => first_num(m, &["ctr"]),
            "cpc" => first_num(m, &["cpc"]),
            "sum" => first_num(m, &["sum", "spend"]),
            "atbs" => first_num(m, &["atbs", "add_to_cart"]),
            "orders" => first_num(m, &["orders"]),
            "cr" => first_num(m, &["cr"]),
            "avgPosition" => first_num(m, &["avg_position", "avgPosition"]),
            _ => String::new(),
        })
        .collect()
}

fn fmt_date(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return sheet_dates::format_date(d);
    }
    sheet_dates::format_wb_timestamp(t)
}

fn first_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        match v.get(*k) {
            Some(Value::String(s)) if !s.is_empty() => return s.clone(),
            Some(Value::Number(n)) => return n.to_string(),
            _ => {}
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
