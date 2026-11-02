//! Поля и строки для `wb_adv_fullstats` (статистика РК по дням/nm).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("advertId", true),
        field("date", true),
        field("nmId", true),
        field("name", true),
        field("views", true),
        field("clicks", true),
        field("ctr", true),
        field("cpc", true),
        field("sum", true),
        field("atbs", true),
        field("orders", true),
        field("cr", true),
        field("shks", true),
        field("sumPrice", true),
        field("canceled", true),
        field("currency", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_fullstats.{name}")),
        selected,
    }
}

pub fn append_fullstats_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    item: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let advert_id = json_num_str(item, "advertId");
    let currency = json_str(item, "currency");
    let days = item
        .get("days")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    if days.is_empty() {
        out.push(row(
            &selected,
            cabinet,
            &advert_id,
            "",
            "",
            "",
            item,
            &currency,
        ));
        return;
    }
    for day in &days {
        let date = format_date_cell(json_str(day, "date"));
        let nms = day
            .get("nms")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        if nms.is_empty() {
            out.push(row(
                &selected,
                cabinet,
                &advert_id,
                &date,
                "",
                "",
                day,
                &currency,
            ));
            continue;
        }
        for nm in &nms {
            let nm_id = json_num_str(nm, "nmId");
            let name = json_str(nm, "name");
            out.push(row(
                &selected,
                cabinet,
                &advert_id,
                &date,
                &nm_id,
                &name,
                nm,
                &currency,
            ));
        }
    }
}

fn row(
    selected: &[&ScriptField],
    cabinet: &str,
    advert_id: &str,
    date: &str,
    nm_id: &str,
    name: &str,
    metrics: &Value,
    currency: &str,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "advertId" => advert_id.to_string(),
            "date" => date.to_string(),
            "nmId" => nm_id.to_string(),
            "name" => name.to_string(),
            "views" => json_num_str(metrics, "views"),
            "clicks" => json_num_str(metrics, "clicks"),
            "ctr" => json_num_str(metrics, "ctr"),
            "cpc" => json_num_str(metrics, "cpc"),
            "sum" => json_num_str(metrics, "sum"),
            "atbs" => json_num_str(metrics, "atbs"),
            "orders" => json_num_str(metrics, "orders"),
            "cr" => json_num_str(metrics, "cr"),
            "shks" => json_num_str(metrics, "shks"),
            "sumPrice" => json_num_str(metrics, "sum_price"),
            "canceled" => json_num_str(metrics, "canceled"),
            "currency" => currency.to_string(),
            _ => String::new(),
        })
        .collect()
}

fn format_date_cell(raw: String) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d") {
        return sheet_dates::format_date(d);
    }
    sheet_dates::format_wb_timestamp(t)
}

fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn json_num_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}
