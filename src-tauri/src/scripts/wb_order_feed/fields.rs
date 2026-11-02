//! DefaultFields `order_feed` (паритет OrderFeedScript.DefaultFields).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.order_feed.cabinet"), true),
        ScriptField::new("snapshotTime", crate::core::i18n::t("fields.order_feed.snapshotTime"), false),
        ScriptField::new("currency", crate::core::i18n::t("fields.order_feed.currency"), false),
        ScriptField::new("nmId", crate::core::i18n::t("fields.order_feed.nmId"), true),
        ScriptField::new("chrtId", crate::core::i18n::t("fields.order_feed.chrtId"), false),
        ScriptField::new("srid", crate::core::i18n::t("fields.order_feed.srid"), true),
        ScriptField::new("createdAt", crate::core::i18n::t("fields.order_feed.createdAt"), true),
        ScriptField::new("updatedAt", crate::core::i18n::t("fields.order_feed.updatedAt"), true),
        ScriptField::new("status", crate::core::i18n::t("fields.order_feed.status"), true),
        ScriptField::new("cancelType", crate::core::i18n::t("fields.order_feed.cancelType"), false),
        ScriptField::new("warehouseName", crate::core::i18n::t("fields.order_feed.warehouseName"), false),
        ScriptField::new("warehouseRegion", crate::core::i18n::t("fields.order_feed.warehouseRegion"), false),
        ScriptField::new("isMp", crate::core::i18n::t("fields.order_feed.isMp"), false),
        ScriptField::new("destinationCity", crate::core::i18n::t("fields.order_feed.destinationCity"), false),
        ScriptField::new("destinationDistrict", crate::core::i18n::t("fields.order_feed.destinationDistrict"), false),
        ScriptField::new("sellerPrice", crate::core::i18n::t("fields.order_feed.sellerPrice"), true),
        ScriptField::new("isB2b", crate::core::i18n::t("fields.order_feed.isB2b"), false),
    ]
}

pub fn build_rows(
    cabinet: &str,
    orders: &[Value],
    snapshot_time: &str,
    currency: &str,
    fields: &[ScriptField],
) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut out = Vec::with_capacity(orders.len() + 1);
    out.push(selected.iter().map(|f| f.title.clone()).collect());
    for o in orders {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(field_value(cabinet, o, snapshot_time, currency, &f.name));
        }
        out.push(row);
    }
    out
}

fn field_value(
    cabinet: &str,
    o: &Value,
    snapshot_time: &str,
    currency: &str,
    name: &str,
) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "snapshotTime" => sheet_dates::format_wb_timestamp(snapshot_time),
        "currency" => currency.to_string(),
        "nmId" | "chrtId" => num_str(o.get(name)),
        "srid" => json_str(o.get("srid")),
        "createdAt" | "updatedAt" => sheet_dates::format_wb_timestamp(&json_str(o.get(name))),
        "status" => translate_status(&json_str(o.get("status"))),
        "cancelType" => translate_cancel(
            &json_str(o.get("status")),
            &json_str(o.get("cancelType")),
        ),
        "warehouseName" | "warehouseRegion" | "destinationCity" | "destinationDistrict" => {
            json_str(o.get(name))
        }
        "isMp" | "isB2b" => format_bool(o.get(name)),
        "sellerPrice" => format_number_ru(o.get("sellerPrice")),
        _ => String::new(),
    }
}

fn json_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

fn num_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn format_bool(v: Option<&Value>) -> String {
    match v {
        Some(Value::Bool(true)) => "Да".into(),
        Some(Value::Bool(false)) => "Нет".into(),
        _ => String::new(),
    }
}

fn format_number_ru(v: Option<&Value>) -> String {
    let Some(n) = v.and_then(|x| x.as_f64()) else {
        return String::new();
    };
    // ru-RU: comma decimal
    format!("{n:.2}").replace('.', ",")
}

fn translate_status(status: &str) -> String {
    match status.trim() {
        "" => String::new(),
        "created" => "оформлен".into(),
        "buyout" => "продан".into(),
        "cancel" => "отменён".into(),
        "return" => "возвращён".into(),
        "returnDefective" => "возвращён по причине брака".into(),
        other => other.to_string(),
    }
}

fn translate_cancel(status: &str, cancel_type: &str) -> String {
    if !status.eq_ignore_ascii_case("cancel") || cancel_type.trim().is_empty() {
        return String::new();
    }
    match cancel_type.trim() {
        "app" => "отказ до получения".into(),
        "receipt" => "отказ при получении".into(),
        "expire" => "истёк срок получения".into(),
        "other" => "техническая отмена".into(),
        other => other.to_string(),
    }
}
