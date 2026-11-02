//! DefaultFields карточек WB (паритет CardsScript.DefaultFields).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use super::docs;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.cards_list.cabinet"), true),
        ScriptField::new("nmID", crate::core::i18n::t("fields.cards_list.nmID"), true),
        ScriptField::new("imtID", crate::core::i18n::t("fields.cards_list.imtID"), true),
        ScriptField::new("nmUUID", crate::core::i18n::t("fields.cards_list.nmUUID"), false),
        ScriptField::new("subjectID", crate::core::i18n::t("fields.cards_list.subjectID"), false),
        ScriptField::new("subjectName", crate::core::i18n::t("fields.cards_list.subjectName"), true),
        ScriptField::new("vendorCode", crate::core::i18n::t("fields.cards_list.vendorCode"), true),
        ScriptField::new("brand", crate::core::i18n::t("fields.cards_list.brand"), true),
        ScriptField::new("title", crate::core::i18n::t("fields.cards_list.title"), true),
        ScriptField::new("description", crate::core::i18n::t("fields.cards_list.description"), false),
        ScriptField::new("needKiz", crate::core::i18n::t("fields.cards_list.needKiz"), false),
        ScriptField::new("photos", crate::core::i18n::t("fields.cards_list.photos"), false),
        ScriptField::new(
            "wholesale.enabled",
            "Предназначена ли карточка товара для оптовой продажи",
            false,
        ),
        ScriptField::new(
            "wholesale.quantum",
            "Количество единиц товара в упаковке",
            false,
        ),
        ScriptField::new("dimensions.length", crate::core::i18n::t("fields.cards_list.dimensions.length"), false),
        ScriptField::new("dimensions.width", crate::core::i18n::t("fields.cards_list.dimensions.width"), false),
        ScriptField::new("dimensions.height", crate::core::i18n::t("fields.cards_list.dimensions.height"), false),
        ScriptField::new("dimensions.weightBrutto", crate::core::i18n::t("fields.cards_list.dimensions.weightBrutto"), false),
        ScriptField::new("isValid", crate::core::i18n::t("fields.cards_list.isValid"), false),
        ScriptField::new("characteristics", crate::core::i18n::t("fields.cards_list.characteristics"), false),
        ScriptField::new("tnved", crate::core::i18n::t("fields.cards_list.tnved"), false),
        ScriptField::new("doc_number", crate::core::i18n::t("fields.cards_list.doc_number"), false),
        ScriptField::new(
            "cert_reg_date",
            "Дата регистрации сертификата/декларации",
            false,
        ),
        ScriptField::new(
            "cert_expire_date",
            "Дата окончания действия сертификата/декларации",
            false,
        ),
        ScriptField::new("cert_check_status", crate::core::i18n::t("fields.cards_list.cert_check_status"), false),
        ScriptField::new("vat_rate", crate::core::i18n::t("fields.cards_list.vat_rate"), false),
        ScriptField::new("sizes.chrtID", crate::core::i18n::t("fields.cards_list.sizes.chrtID"), false),
        ScriptField::new("sizes.techSize", crate::core::i18n::t("fields.cards_list.sizes.techSize"), false),
        ScriptField::new("sizes.wbSize", crate::core::i18n::t("fields.cards_list.sizes.wbSize"), false),
        ScriptField::new("sizes.skus", crate::core::i18n::t("fields.cards_list.sizes.skus"), false),
        ScriptField::new("tags.id", crate::core::i18n::t("fields.cards_list.tags.id"), false),
        ScriptField::new("tags.name", crate::core::i18n::t("fields.cards_list.tags.name"), false),
        ScriptField::new("tags.color", crate::core::i18n::t("fields.cards_list.tags.color"), false),
        ScriptField::new("createdAt", crate::core::i18n::t("fields.cards_list.createdAt"), true),
        ScriptField::new("updatedAt", crate::core::i18n::t("fields.cards_list.updatedAt"), true),
    ]
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
        "nmID" => json_str(card, &["nmID", "nmId"]),
        "imtID" => json_str(card, &["imtID", "imtId"]),
        "nmUUID" => json_str(card, &["nmUUID", "nmUuid"]),
        "subjectID" => json_str(card, &["subjectID", "subjectId"]),
        "subjectName" => json_str(card, &["subjectName"]),
        "vendorCode" => json_str(card, &["vendorCode"]),
        "brand" => json_str(card, &["brand"]),
        "title" => json_str(card, &["title"]),
        "description" => json_str(card, &["description"]),
        "needKiz" => json_str(card, &["needKiz"]),
        "photos" => photos_cell(card),
        "wholesale.enabled" => path_str(card, &["wholesale", "enabled"]),
        "wholesale.quantum" => path_str(card, &["wholesale", "quantum"]),
        "dimensions.length" => path_str(card, &["dimensions", "length"]),
        "dimensions.width" => path_str(card, &["dimensions", "width"]),
        "dimensions.height" => path_str(card, &["dimensions", "height"]),
        "dimensions.weightBrutto" => path_str(card, &["dimensions", "weightBrutto"]),
        "isValid" => json_str(card, &["isValid"]),
        "characteristics" => characteristics_cell(card),
        // CardsScript.FieldMapping — не path_dotted (в JSON нет плоских tnved/doc_*).
        "tnved" => docs::tnved_pair(card),
        "doc_number" | "decl_number" | "cert_number" => docs::document_numbers(card),
        "cert_reg_date" => docs::document_start_dates(card),
        "cert_expire_date" => docs::document_end_dates(card),
        "cert_check_status" => docs::document_check_statuses(card),
        "createdAt" => sheet_dates::format_wb_timestamp(&json_str(card, &["createdAt"])),
        "updatedAt" => sheet_dates::format_wb_timestamp(&json_str(card, &["updatedAt"])),
        // CardsScript: chrtID через `;` (остатки FBS парсят ParseChrtIdList).
        "sizes.chrtID" => sizes_join_sep(card, "chrtID", ";"),
        "sizes.techSize" => sizes_join_sep(card, "techSize", ", "),
        "sizes.wbSize" => sizes_join_sep(card, "wbSize", ", "),
        "sizes.skus" => sizes_skus(card),
        "tags.id" => tags_join(card, "id"),
        "tags.name" => tags_join(card, "name"),
        "tags.color" => tags_join(card, "color"),
        other => path_dotted(card, other),
    }
}

fn photos_cell(card: &Value) -> String {
    let Some(arr) = card.get("photos").and_then(|p| p.as_array()) else {
        return String::new();
    };
    arr.iter()
        .take(30)
        .filter_map(|p| {
            p.get("big")
                .or_else(|| p.get("c246x328"))
                .or_else(|| p.get("sq"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn characteristics_cell(card: &Value) -> String {
    let Some(arr) = card.get("characteristics").and_then(|c| c.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|ch| {
            let name = ch.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let val = ch
                .get("value")
                .map(|v| match v {
                    Value::Array(a) => a
                        .iter()
                        .map(|x| x.as_str().unwrap_or("").to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            if name.is_empty() && val.is_empty() {
                None
            } else {
                Some(format!("{name}: {val}"))
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
        .map(|s| json_str(s, &[key]))
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
        .map(|t| json_str(t, &[key]))
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

fn json_str(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(x) = v.get(*k) {
            return match x {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string().trim_matches('"').to_string(),
            };
        }
    }
    String::new()
}
