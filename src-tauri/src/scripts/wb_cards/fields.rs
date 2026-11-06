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

//! DefaultFields карточек WB — schema /content/v2/get/cards/list + удобства StatHive.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use super::docs;
use serde_json::Value;

fn f(name: &str, title_key: &str, selected: bool) -> ScriptField {
    ScriptField::new(name, crate::core::i18n::t(title_key), selected)
}

const PHOTO_SLOTS: usize = 30;

pub fn default_fields() -> Vec<ScriptField> {
    let mut out = vec![
        f("cabinet", "fields.cards_list.cabinet", true),
        f("nmID", "fields.cards_list.nmID", true),
        f("imtID", "fields.cards_list.imtID", true),
        f("nmUUID", "fields.cards_list.nmUUID", false),
        f("subjectID", "fields.cards_list.subjectID", false),
        f("subjectName", "fields.cards_list.subjectName", true),
        f("vendorCode", "fields.cards_list.vendorCode", true),
        f("brand", "fields.cards_list.brand", true),
        f("title", "fields.cards_list.title", true),
        f("gtin", "fields.cards_list.gtin", false),
        f("description", "fields.cards_list.description", false),
        f("needKiz", "fields.cards_list.needKiz", false),
        f("kizMarked", "fields.cards_list.kizMarked", false),
        f("video", "fields.cards_list.video", false),
        f("wholesale.enabled", "fields.cards_list.wholesale.enabled", false),
        f("wholesale.quantum", "fields.cards_list.wholesale.quantum", false),
        f("dimensions.length", "fields.cards_list.dimensions.length", false),
        f("dimensions.width", "fields.cards_list.dimensions.width", false),
        f("dimensions.height", "fields.cards_list.dimensions.height", false),
        f(
            "dimensions.weightBrutto",
            "fields.cards_list.dimensions.weightBrutto",
            false,
        ),
        f("dimensions.isValid", "fields.cards_list.dimensions.isValid", false),
        f("characteristics", "fields.cards_list.characteristics", false),
        f("tnved", "fields.cards_list.tnved", false),
        f("vat_rate", "fields.cards_list.vat_rate", false),
        f("doc_number", "fields.cards_list.doc_number", false),
        f("cert_reg_date", "fields.cards_list.cert_reg_date", false),
        f("cert_expire_date", "fields.cards_list.cert_expire_date", false),
        f("cert_check_status", "fields.cards_list.cert_check_status", false),
        f(
            "documents.excludeDocuments",
            "fields.cards_list.documents.excludeDocuments",
            false,
        ),
        f(
            "documents.overallVerdict.isFullyChecked",
            "fields.cards_list.documents.overallVerdict.isFullyChecked",
            false,
        ),
        f(
            "documents.overallVerdict.status",
            "fields.cards_list.documents.overallVerdict.status",
            false,
        ),
        f(
            "documents.overallVerdict.reason",
            "fields.cards_list.documents.overallVerdict.reason",
            false,
        ),
        f(
            "documents.overallVerdict.createdAt",
            "fields.cards_list.documents.overallVerdict.createdAt",
            false,
        ),
        f("documents.items.id", "fields.cards_list.documents.items.id", false),
        f("documents.items.type", "fields.cards_list.documents.items.type", false),
        f(
            "documents.items.number",
            "fields.cards_list.documents.items.number",
            false,
        ),
        f(
            "documents.items.productNumber",
            "fields.cards_list.documents.items.productNumber",
            false,
        ),
        f(
            "documents.items.tradeName",
            "fields.cards_list.documents.items.tradeName",
            false,
        ),
        f(
            "documents.items.applicant",
            "fields.cards_list.documents.items.applicant",
            false,
        ),
        f(
            "documents.items.startDate",
            "fields.cards_list.documents.items.startDate",
            false,
        ),
        f(
            "documents.items.endDate",
            "fields.cards_list.documents.items.endDate",
            false,
        ),
        f(
            "documents.items.isEndless",
            "fields.cards_list.documents.items.isEndless",
            false,
        ),
        f(
            "documents.items.createdAt",
            "fields.cards_list.documents.items.createdAt",
            false,
        ),
        f(
            "documents.items.verdict.verified",
            "fields.cards_list.documents.items.verdict.verified",
            false,
        ),
        f(
            "documents.items.verdict.status",
            "fields.cards_list.documents.items.verdict.status",
            false,
        ),
        f(
            "documents.items.verdict.reason",
            "fields.cards_list.documents.items.verdict.reason",
            false,
        ),
        f(
            "documents.items.verdict.createdAt",
            "fields.cards_list.documents.items.verdict.createdAt",
            false,
        ),
        f(
            "documents.items.verdict.additionalData",
            "fields.cards_list.documents.items.verdict.additionalData",
            false,
        ),
        f("sizes.chrtID", "fields.cards_list.sizes.chrtID", false),
        f("sizes.techSize", "fields.cards_list.sizes.techSize", false),
        f("sizes.wbSize", "fields.cards_list.sizes.wbSize", false),
        f("sizes.skus", "fields.cards_list.sizes.skus", false),
        f("tags.id", "fields.cards_list.tags.id", false),
        f("tags.name", "fields.cards_list.tags.name", false),
        f("tags.color", "fields.cards_list.tags.color", false),
        f("createdAt", "fields.cards_list.createdAt", true),
        f("updatedAt", "fields.cards_list.updatedAt", true),
    ];
    // Photos last in template field order (up to 30 big URLs).
    let photo_label = crate::core::i18n::t("fields.cards_list.photo");
    for i in 1..=PHOTO_SLOTS {
        out.push(ScriptField {
            name: format!("photo_{i}"),
            title: format!("{photo_label} {i}"),
            selected: false,
        });
    }
    out
}

pub fn build_rows(
    cabinet: &str,
    cards: &[Value],
    fields: &[ScriptField],
) -> Vec<Vec<String>> {
    let cols: Vec<ScriptField> = {
        let selected: Vec<ScriptField> = fields.iter().filter(|f| f.selected).cloned().collect();
        if selected.is_empty() {
            default_fields()
                .into_iter()
                .filter(|f| f.selected)
                .collect()
        } else {
            selected
        }
    };

    let mut rows = Vec::with_capacity(cards.len() + 1);
    rows.push(cols.iter().map(|f| f.title.clone()).collect());
    for c in cards {
        rows.push(
            cols.iter()
                .map(|f| cell_value(cabinet, c, &f.name))
                .collect(),
        );
    }
    rows
}

fn cell_value(cabinet: &str, card: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmID" => json_str(card, "nmID"),
        "imtID" => json_str(card, "imtID"),
        "nmUUID" => json_str(card, "nmUUID"),
        "subjectID" => json_str(card, "subjectID"),
        "subjectName" => json_str(card, "subjectName"),
        "vendorCode" => json_str(card, "vendorCode"),
        "brand" => json_str(card, "brand"),
        "title" => json_str(card, "title"),
        "gtin" => json_str(card, "gtin"),
        "description" => json_str(card, "description"),
        "needKiz" => json_str(card, "needKiz"),
        "kizMarked" => json_str(card, "kizMarked"),
        "video" => json_str(card, "video"),
        "wholesale.enabled" => path_str(card, &["wholesale", "enabled"]),
        "wholesale.quantum" => path_str(card, &["wholesale", "quantum"]),
        "dimensions.length" => path_str(card, &["dimensions", "length"]),
        "dimensions.width" => path_str(card, &["dimensions", "width"]),
        "dimensions.height" => path_str(card, &["dimensions", "height"]),
        "dimensions.weightBrutto" => path_str(card, &["dimensions", "weightBrutto"]),
        "dimensions.isValid" => path_str(card, &["dimensions", "isValid"]),
        "characteristics" => characteristics_cell(card),
        "tnved" => docs::tnved_pair(card),
        "vat_rate" => docs::vat_rate_from_charcs(card),
        "doc_number" => docs::document_numbers(card),
        "cert_reg_date" => docs::document_start_dates(card),
        "cert_expire_date" => docs::document_end_dates(card),
        "cert_check_status" => docs::document_check_statuses(card),
        "documents.excludeDocuments" => docs::exclude_documents(card),
        "documents.overallVerdict.isFullyChecked" => {
            docs::overall_verdict_field(card, "isFullyChecked")
        }
        "documents.overallVerdict.status" => docs::overall_verdict_field(card, "status"),
        "documents.overallVerdict.reason" => docs::overall_verdict_field(card, "reason"),
        "documents.overallVerdict.createdAt" => docs::overall_verdict_field(card, "createdAt"),
        "documents.items.id" => docs::items_field_join(card, "id"),
        "documents.items.type" => docs::items_field_join(card, "type"),
        "documents.items.number" => docs::items_field_join(card, "number"),
        "documents.items.productNumber" => docs::items_field_join(card, "productNumber"),
        "documents.items.tradeName" => docs::items_field_join(card, "tradeName"),
        "documents.items.applicant" => docs::items_field_join(card, "applicant"),
        "documents.items.startDate" => docs::items_field_join(card, "startDate"),
        "documents.items.endDate" => docs::items_field_join(card, "endDate"),
        "documents.items.isEndless" => docs::items_field_join(card, "isEndless"),
        "documents.items.createdAt" => docs::items_field_join(card, "createdAt"),
        "documents.items.verdict.verified" => docs::items_field_join(card, "verdict.verified"),
        "documents.items.verdict.status" => docs::items_field_join(card, "verdict.status"),
        "documents.items.verdict.reason" => docs::items_field_join(card, "verdict.reason"),
        "documents.items.verdict.createdAt" => docs::items_field_join(card, "verdict.createdAt"),
        "documents.items.verdict.additionalData" => {
            docs::items_field_join(card, "verdict.additionalData")
        }
        "createdAt" => sheet_dates::format_wb_timestamp(&json_str(card, "createdAt")),
        "updatedAt" => sheet_dates::format_wb_timestamp(&json_str(card, "updatedAt")),
        "sizes.chrtID" => sizes_join_sep(card, "chrtID", ";"),
        "sizes.techSize" => sizes_join_sep(card, "techSize", ", "),
        "sizes.wbSize" => sizes_join_sep(card, "wbSize", ", "),
        "sizes.skus" => sizes_skus(card),
        "tags.id" => tags_join(card, "id"),
        "tags.name" => tags_join(card, "name"),
        "tags.color" => tags_join(card, "color"),
        other => {
            if let Some(n) = other.strip_prefix("photo_") {
                if let Ok(idx) = n.parse::<usize>() {
                    return photo_big_at(card, idx);
                }
            }
            path_dotted(card, other)
        }
    }
}

/// `photo_1` → photos[0].big (1-based index).
fn photo_big_at(card: &Value, one_based: usize) -> String {
    if one_based == 0 || one_based > PHOTO_SLOTS {
        return String::new();
    }
    card.get("photos")
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(one_based - 1))
        .and_then(|p| p.get("big"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn characteristics_cell(card: &Value) -> String {
    let Some(arr) = card.get("characteristics").and_then(|c| c.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|ch| {
            let id = json_str(ch, "id");
            let name = ch.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let val = ch
                .get("value")
                .map(|v| match v {
                    Value::Array(a) => a
                        .iter()
                        .map(|x| match x {
                            Value::String(s) => s.clone(),
                            other => other.to_string().trim_matches('"').to_string(),
                        })
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(", "),
                    Value::String(s) => s.clone(),
                    other => other.to_string().trim_matches('"').to_string(),
                })
                .unwrap_or_default();
            if id.is_empty() && name.is_empty() && val.is_empty() {
                None
            } else {
                Some(format!("{id}:{name}:{val}"))
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn sizes_join_sep(card: &Value, key: &str, sep: &str) -> String {
    let Some(arr) = card.get("sizes").and_then(|s| s.as_array()) else {
        return String::new();
    };
    arr.iter()
        .map(|s| json_str(s, key))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}

fn sizes_skus(card: &Value) -> String {
    let Some(arr) = card.get("sizes").and_then(|s| s.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|s| s.get("skus").and_then(|k| k.as_array()))
        .flatten()
        .filter_map(|x| x.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn tags_join(card: &Value, key: &str) -> String {
    let Some(arr) = card.get("tags").and_then(|t| t.as_array()) else {
        return String::new();
    };
    arr.iter()
        .map(|t| json_str(t, key))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn path_dotted(card: &Value, dotted: &str) -> String {
    let parts: Vec<&str> = dotted.split('.').collect();
    path_str(card, &parts)
}

fn path_str(root: &Value, parts: &[&str]) -> String {
    let mut cur = root;
    for (i, p) in parts.iter().enumerate() {
        match cur.get(*p) {
            Some(next) if i + 1 == parts.len() => {
                return match next {
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Null => String::new(),
                    other => other.to_string().trim_matches('"').to_string(),
                };
            }
            Some(next) => cur = next,
            None => return String::new(),
        }
    }
    String::new()
}

fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_gtin_and_dimensions_is_valid() {
        let card = json!({
            "gtin": "04607009520001",
            "kizMarked": true,
            "video": "https://example/video.m3u8",
            "dimensions": {
                "length": 55,
                "isValid": false
            }
        });
        assert_eq!(cell_value("cab", &card, "gtin"), "04607009520001");
        assert_eq!(cell_value("cab", &card, "kizMarked"), "true");
        assert_eq!(cell_value("cab", &card, "video"), "https://example/video.m3u8");
        assert_eq!(cell_value("cab", &card, "dimensions.isValid"), "false");
        assert!(path_str(&card, &["isValid"]).is_empty());
    }

    #[test]
    fn characteristics_include_id() {
        let card = json!({
            "characteristics": [
                { "id": 14177449, "name": "Цвет", "value": ["красно-сиреневый"] }
            ]
        });
        let s = cell_value("c", &card, "characteristics");
        assert!(s.contains("14177449:Цвет:красно-сиреневый"));
    }

    #[test]
    fn photo_columns_use_big_and_sit_last() {
        let fields = default_fields();
        let last = &fields[fields.len() - PHOTO_SLOTS..];
        assert_eq!(last[0].name, "photo_1");
        assert_eq!(last[29].name, "photo_30");
        let card = json!({
            "photos": [
                { "big": "https://example/1.webp", "tm": "https://example/1tm.webp" },
                { "big": "https://example/2.webp" }
            ]
        });
        assert_eq!(cell_value("c", &card, "photo_1"), "https://example/1.webp");
        assert_eq!(cell_value("c", &card, "photo_2"), "https://example/2.webp");
        assert!(cell_value("c", &card, "photo_3").is_empty());
    }
}
