//! Поля и строки для `wb_adv_campaigns_stats_new` (лист «Инфо»).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("advertId", true),
        field("bidType", true),
        field("status", true),
        field("statusName", true),
        field("paymentType", true),
        field("campaignName", true),
        field("placementsSearch", true),
        field("placementsRecommendations", true),
        field("created", true),
        field("updated", true),
        field("started", true),
        field("deleted", true),
        field("nmId", true),
        field("subjectId", true),
        field("subjectName", true),
        field("bidSearch", true),
        field("bidRecommendations", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.wb_adv_campaigns_stats_new.{name}")),
        selected,
    }
}

pub fn map_status_name(status: &str) -> String {
    match status.trim() {
        "-1" => "Удалена (в процессе удаления)".into(),
        "4" => "Готова к запуску".into(),
        "7" => "Завершена".into(),
        "8" => "Отменена".into(),
        "9" => "Активна".into(),
        "11" => "На паузе".into(),
        s if s.is_empty() => String::new(),
        s => format!("Статус {s}"),
    }
}

pub fn append_advert_rows(
    out: &mut Vec<Vec<String>>,
    fields: &[ScriptField],
    cabinet: &str,
    advert: &Value,
) {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return;
    }
    let advert_id = json_num_str(advert, "id");
    let bid_type = json_str(advert, "bid_type");
    let status = json_num_str(advert, "status");
    let status_name = map_status_name(&status);
    let settings = advert.get("settings");
    let payment_type = settings.map(|s| json_str(s, "payment_type")).unwrap_or_default();
    let campaign_name = settings.map(|s| json_str(s, "name")).unwrap_or_default();
    let placements = settings.and_then(|s| s.get("placements"));
    let placements_search = placements
        .and_then(|p| p.get("search"))
        .map(|v| bool_str(v))
        .unwrap_or_default();
    let placements_rec = placements
        .and_then(|p| p.get("recommendations"))
        .map(|v| bool_str(v))
        .unwrap_or_default();
    let timestamps = advert.get("timestamps");
    let created = fmt_ts(timestamps.map(|t| json_str(t, "created")).unwrap_or_default());
    let updated = fmt_ts(timestamps.map(|t| json_str(t, "updated")).unwrap_or_default());
    let started = fmt_ts(timestamps.map(|t| json_str(t, "started")).unwrap_or_default());
    let deleted = fmt_ts(timestamps.map(|t| json_str(t, "deleted")).unwrap_or_default());

    let nm_settings = advert
        .get("nm_settings")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    if nm_settings.is_empty() {
        out.push(build_row(
            &selected,
            cabinet,
            &advert_id,
            &bid_type,
            &status,
            &status_name,
            &payment_type,
            &campaign_name,
            &placements_search,
            &placements_rec,
            &created,
            &updated,
            &started,
            &deleted,
            "",
            "",
            "",
            "0",
            "0",
        ));
        return;
    }

    for nm in &nm_settings {
        let nm_id = json_num_str(nm, "nm_id");
        let subject = nm.get("subject");
        let subject_id = subject.map(|s| json_num_str(s, "id")).unwrap_or_default();
        let subject_name = subject.map(|s| json_str(s, "name")).unwrap_or_default();
        let bids = nm.get("bids_kopecks");
        let bid_search = kopecks_to_rub(bids.and_then(|b| b.get("search")));
        let bid_rec = kopecks_to_rub(bids.and_then(|b| b.get("recommendations")));
        out.push(build_row(
            &selected,
            cabinet,
            &advert_id,
            &bid_type,
            &status,
            &status_name,
            &payment_type,
            &campaign_name,
            &placements_search,
            &placements_rec,
            &created,
            &updated,
            &started,
            &deleted,
            &nm_id,
            &subject_id,
            &subject_name,
            &bid_search,
            &bid_rec,
        ));
    }
}

fn build_row(
    selected: &[&ScriptField],
    cabinet: &str,
    advert_id: &str,
    bid_type: &str,
    status: &str,
    status_name: &str,
    payment_type: &str,
    campaign_name: &str,
    placements_search: &str,
    placements_rec: &str,
    created: &str,
    updated: &str,
    started: &str,
    deleted: &str,
    nm_id: &str,
    subject_id: &str,
    subject_name: &str,
    bid_search: &str,
    bid_rec: &str,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "advertId" => advert_id.to_string(),
            "bidType" => bid_type.to_string(),
            "status" => status.to_string(),
            "statusName" => status_name.to_string(),
            "paymentType" => payment_type.to_string(),
            "campaignName" => campaign_name.to_string(),
            "placementsSearch" => placements_search.to_string(),
            "placementsRecommendations" => placements_rec.to_string(),
            "created" => created.to_string(),
            "updated" => updated.to_string(),
            "started" => started.to_string(),
            "deleted" => deleted.to_string(),
            "nmId" => nm_id.to_string(),
            "subjectId" => subject_id.to_string(),
            "subjectName" => subject_name.to_string(),
            "bidSearch" => bid_search.to_string(),
            "bidRecommendations" => bid_rec.to_string(),
            _ => String::new(),
        })
        .collect()
}

fn fmt_ts(raw: String) -> String {
    if raw.trim().is_empty() {
        raw
    } else {
        sheet_dates::format_wb_timestamp(&raw)
    }
}

fn kopecks_to_rub(v: Option<&Value>) -> String {
    let kop = match v {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    };
    let rub = (kop as f64) / 100.0;
    if (rub - rub.round()).abs() < f64::EPSILON {
        format!("{}", rub as i64)
    } else {
        format!("{rub}")
    }
}

fn bool_str(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_num_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}
