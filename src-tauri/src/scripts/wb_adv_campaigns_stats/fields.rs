// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Поля и строки для `wb_adv_campaigns_stats_new` — `GET /api/advert/v3/adverts`.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("advertId", true),
        field("bidType", true),
        field("currency", false),
        field("status", true),
        field("statusName", true),
        field("paymentType", true),
        field("campaignName", true),
        field("placementsSearch", true),
        field("placementsRecommendations", true),
        field("placementsCombined", true),
        field("canChangeNms", false),
        field("created", true),
        field("updated", true),
        field("started", true),
        field("ended", true),
        field("nmId", true),
        field("subjectId", true),
        field("subjectName", true),
        field("bidSearch", true),
        field("bidRecommendations", true),
        field("bidCombined", true),
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
        "4" => "Готова к запуску".into(),
        "7" => "Завершена".into(),
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
    let advert_id = json_num_str(advert, "advertId");
    let currency = json_str(advert, "currency");
    let status = json_num_str(advert, "status");
    let status_name = map_status_name(&status);
    let settings = advert.get("settings");
    let payment_type = settings
        .map(|s| json_str(s, "paymentType"))
        .unwrap_or_default();
    let campaign_name = settings.map(|s| json_str(s, "name")).unwrap_or_default();
    let placements = settings
        .and_then(|s| s.get("placements"))
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();
    let place_search = placement_has(&placements, "search");
    let place_rec = placement_has(&placements, "recommendations");
    let place_combined = placement_has(&placements, "combined");
    let bid_type = derive_bid_type(&placements);
    let can_change_nms = advert
        .pointer("/restrictions/canChangeNms")
        .map(bool_str)
        .unwrap_or_default();
    let timestamps = advert.get("timestamps");
    let created = fmt_ts(timestamps.map(|t| json_str(t, "created")).unwrap_or_default());
    let updated = fmt_ts(timestamps.map(|t| json_str(t, "updated")).unwrap_or_default());
    let started = fmt_ts(timestamps.map(|t| json_str(t, "started")).unwrap_or_default());
    let ended = fmt_ts(timestamps.map(|t| json_str(t, "ended")).unwrap_or_default());

    let nms = advert
        .get("nms")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    if nms.is_empty() {
        out.push(build_row(
            &selected,
            cabinet,
            &advert_id,
            &bid_type,
            &currency,
            &status,
            &status_name,
            &payment_type,
            &campaign_name,
            &place_search,
            &place_rec,
            &place_combined,
            &can_change_nms,
            &created,
            &updated,
            &started,
            &ended,
            "",
            "",
            "",
            "0",
            "0",
            "0",
        ));
        return;
    }

    for nm in &nms {
        let nm_id = json_num_str(nm, "nmId");
        let subject = nm.get("subject");
        let subject_id = subject.map(|s| json_num_str(s, "id")).unwrap_or_default();
        let subject_name = subject.map(|s| json_str(s, "name")).unwrap_or_default();
        let (bid_search, bid_rec, bid_combined) = bids_from_array(nm.get("bids"));
        out.push(build_row(
            &selected,
            cabinet,
            &advert_id,
            &bid_type,
            &currency,
            &status,
            &status_name,
            &payment_type,
            &campaign_name,
            &place_search,
            &place_rec,
            &place_combined,
            &can_change_nms,
            &created,
            &updated,
            &started,
            &ended,
            &nm_id,
            &subject_id,
            &subject_name,
            &bid_search,
            &bid_rec,
            &bid_combined,
        ));
    }
}

fn build_row(
    selected: &[&ScriptField],
    cabinet: &str,
    advert_id: &str,
    bid_type: &str,
    currency: &str,
    status: &str,
    status_name: &str,
    payment_type: &str,
    campaign_name: &str,
    placements_search: &str,
    placements_rec: &str,
    placements_combined: &str,
    can_change_nms: &str,
    created: &str,
    updated: &str,
    started: &str,
    ended: &str,
    nm_id: &str,
    subject_id: &str,
    subject_name: &str,
    bid_search: &str,
    bid_rec: &str,
    bid_combined: &str,
) -> Vec<String> {
    selected
        .iter()
        .map(|f| match f.name.as_str() {
            "cabinet" => cabinet.to_string(),
            "advertId" => advert_id.to_string(),
            "bidType" => bid_type.to_string(),
            "currency" => currency.to_string(),
            "status" => status.to_string(),
            "statusName" => status_name.to_string(),
            "paymentType" => payment_type.to_string(),
            "campaignName" => campaign_name.to_string(),
            "placementsSearch" => placements_search.to_string(),
            "placementsRecommendations" => placements_rec.to_string(),
            "placementsCombined" => placements_combined.to_string(),
            "canChangeNms" => can_change_nms.to_string(),
            "created" => created.to_string(),
            "updated" => updated.to_string(),
            "started" => started.to_string(),
            "ended" => ended.to_string(),
            "nmId" => nm_id.to_string(),
            "subjectId" => subject_id.to_string(),
            "subjectName" => subject_name.to_string(),
            "bidSearch" => bid_search.to_string(),
            "bidRecommendations" => bid_rec.to_string(),
            "bidCombined" => bid_combined.to_string(),
            _ => String::new(),
        })
        .collect()
}

fn placement_has(placements: &[Value], kind: &str) -> String {
    let ok = placements.iter().any(|v| v.as_str() == Some(kind));
    ok.to_string()
}

fn derive_bid_type(placements: &[Value]) -> String {
    let mut kinds: Vec<&str> = placements.iter().filter_map(|v| v.as_str()).collect();
    if kinds.is_empty() {
        return String::new();
    }
    kinds.sort_unstable();
    kinds.dedup();
    if kinds.iter().any(|k| *k == "combined") {
        "combined".into()
    } else {
        kinds.join(",")
    }
}

fn bids_from_array(bids: Option<&Value>) -> (String, String, String) {
    let mut search = 0i64;
    let mut rec = 0i64;
    let mut combined = 0i64;
    let Some(arr) = bids.and_then(|b| b.as_array()) else {
        return (
            kopecks_to_rub_i64(0),
            kopecks_to_rub_i64(0),
            kopecks_to_rub_i64(0),
        );
    };
    for b in arr {
        let typ = json_str(b, "type");
        let val = match b.get("value") {
            Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
            Some(Value::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        };
        match typ.as_str() {
            "search" => search = val,
            "recommendations" | "recommendation" => rec = val,
            "combined" => combined = val,
            _ => {}
        }
    }
    (
        kopecks_to_rub_i64(search),
        kopecks_to_rub_i64(rec),
        kopecks_to_rub_i64(combined),
    )
}

fn fmt_ts(raw: String) -> String {
    if raw.trim().is_empty() {
        raw
    } else {
        sheet_dates::format_wb_timestamp(&raw)
    }
}

fn kopecks_to_rub_i64(kop: i64) -> String {
    let rub = (kop as f64) / 100.0;
    if (rub - rub.round()).abs() < f64::EPSILON {
        format!("{}", rub as i64)
    } else {
        // USER_ENTERED + RU locale: "2,29" → number; "2.29" stays text.
        format!("{rub}").replace('.', ",")
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_v3_advert_sample() {
        let advert = json!({
            "advertId": 12345,
            "status": 9,
            "settings": {
                "name": "Моя кампания",
                "paymentType": "cpm",
                "placements": ["combined"]
            },
            "nms": [{
                "nmId": 12345678,
                "subject": { "id": 270, "name": "Парашюты" },
                "bids": [
                    { "type": "search", "value": 65000 },
                    { "type": "combined", "value": 229 }
                ]
            }],
            "timestamps": {
                "created": "2024-01-01T10:00:00+03:00",
                "updated": "2024-01-02T15:30:00+03:00",
                "started": "2024-01-01T10:05:00+03:00",
                "ended": null
            },
            "currency": "RUB",
            "restrictions": { "canChangeNms": true }
        });
        let mut fields = default_fields();
        for f in &mut fields {
            f.selected = true;
        }
        let mut rows = Vec::new();
        append_advert_rows(&mut rows, &fields, "CabA", &advert);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        let idx = |name: &str| {
            fields
                .iter()
                .position(|f| f.name == name)
                .expect(name)
        };
        assert_eq!(row[idx("advertId")], "12345");
        assert_eq!(row[idx("bidType")], "combined");
        assert_eq!(row[idx("placementsCombined")], "true");
        assert_eq!(row[idx("placementsSearch")], "false");
        assert_eq!(row[idx("nmId")], "12345678");
        assert_eq!(row[idx("bidSearch")], "650");
        assert_eq!(row[idx("bidCombined")], "2,29");
        assert_eq!(row[idx("ended")], "");
        assert_eq!(row[idx("canChangeNms")], "true");
        assert_eq!(row[idx("statusName")], "Активна");
    }
}
