//! DefaultFields корзины WB (паритет CardsTrashScript.DefaultFields).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.cards_trash.cabinet"), true),
        ScriptField::new("nmID", crate::core::i18n::t("fields.cards_trash.nmID"), true),
        ScriptField::new("imtID", crate::core::i18n::t("fields.cards_trash.imtID"), false),
        ScriptField::new("nmUUID", crate::core::i18n::t("fields.cards_trash.nmUUID"), false),
        ScriptField::new("subjectID", crate::core::i18n::t("fields.cards_trash.subjectID"), false),
        ScriptField::new("subjectName", crate::core::i18n::t("fields.cards_trash.subjectName"), true),
        ScriptField::new("vendorCode", crate::core::i18n::t("fields.cards_trash.vendorCode"), true),
        ScriptField::new("brand", crate::core::i18n::t("fields.cards_trash.brand"), true),
        ScriptField::new("title", crate::core::i18n::t("fields.cards_trash.title"), true),
        ScriptField::new("description", crate::core::i18n::t("fields.cards_trash.description"), false),
        ScriptField::new("photos", crate::core::i18n::t("fields.cards_trash.photos"), false),
        ScriptField::new("video", crate::core::i18n::t("fields.cards_trash.video"), false),
        ScriptField::new("wholesale.enabled", crate::core::i18n::t("fields.cards_trash.wholesale.enabled"), false),
        ScriptField::new("wholesale.quantum", crate::core::i18n::t("fields.cards_trash.wholesale.quantum"), false),
        ScriptField::new("dimensions.length", crate::core::i18n::t("fields.cards_trash.dimensions.length"), false),
        ScriptField::new("dimensions.width", crate::core::i18n::t("fields.cards_trash.dimensions.width"), false),
        ScriptField::new("dimensions.height", crate::core::i18n::t("fields.cards_trash.dimensions.height"), false),
        ScriptField::new("dimensions.weightBrutto", crate::core::i18n::t("fields.cards_trash.dimensions.weightBrutto"), false),
        ScriptField::new("isValid", crate::core::i18n::t("fields.cards_trash.isValid"), false),
        ScriptField::new("sizes.chrtID", crate::core::i18n::t("fields.cards_trash.sizes.chrtID"), false),
        ScriptField::new("sizes.techSize", crate::core::i18n::t("fields.cards_trash.sizes.techSize"), false),
        ScriptField::new("sizes.wbSize", crate::core::i18n::t("fields.cards_trash.sizes.wbSize"), false),
        ScriptField::new("sizes.skus", crate::core::i18n::t("fields.cards_trash.sizes.skus"), false),
        ScriptField::new("createdAt", crate::core::i18n::t("fields.cards_trash.createdAt"), true),
        ScriptField::new("trashedAt", crate::core::i18n::t("fields.cards_trash.trashedAt"), true),
    ]
}

pub fn build_rows(cabinet: &str, cards: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let cols: Vec<ScriptField> = {
        let selected: Vec<_> = fields.iter().filter(|f| f.selected).cloned().collect();
        if selected.is_empty() {
            default_fields().into_iter().filter(|f| f.selected).collect()
        } else {
            selected
        }
    };
    let mut rows = Vec::with_capacity(cards.len() + 1);
    rows.push(cols.iter().map(|f| f.title.clone()).collect());
    for c in cards {
        rows.push(cols.iter().map(|f| cell(cabinet, c, &f.name)).collect());
    }
    rows
}

fn cell(cabinet: &str, card: &Value, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "nmID" => jstr(card, &["nmID", "nmId"]),
        "imtID" => jstr(card, &["imtID", "imtId"]),
        "nmUUID" => jstr(card, &["nmUUID", "nmUuid"]),
        "subjectID" => jstr(card, &["subjectID", "subjectId"]),
        "subjectName" => jstr(card, &["subjectName"]),
        "vendorCode" => jstr(card, &["vendorCode"]),
        "brand" => jstr(card, &["brand"]),
        "title" => jstr(card, &["title"]),
        "description" => jstr(card, &["description"]),
        "photos" => photos(card),
        "video" => jstr(card, &["video"]),
        "wholesale.enabled" => path(card, &["wholesale", "enabled"]),
        "wholesale.quantum" => path(card, &["wholesale", "quantum"]),
        "dimensions.length" => path(card, &["dimensions", "length"]),
        "dimensions.width" => path(card, &["dimensions", "width"]),
        "dimensions.height" => path(card, &["dimensions", "height"]),
        "dimensions.weightBrutto" => path(card, &["dimensions", "weightBrutto"]),
        "isValid" => jstr(card, &["isValid"]),
        "sizes.chrtID" => sizes_join(card, "chrtID", ";"),
        "sizes.techSize" => sizes_join(card, "techSize", ";"),
        "sizes.wbSize" => sizes_join(card, "wbSize", ";"),
        "sizes.skus" => sizes_skus(card),
        "createdAt" => sheet_dates::format_wb_timestamp(&jstr(card, &["createdAt"])),
        "trashedAt" => sheet_dates::format_wb_timestamp(&jstr(card, &["trashedAt"])),
        _ => String::new(),
    }
}

fn jstr(v: &Value, keys: &[&str]) -> String {
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

fn path(v: &Value, keys: &[&str]) -> String {
    let mut cur = v;
    for k in keys {
        match cur.get(*k) {
            Some(n) => cur = n,
            None => return String::new(),
        }
    }
    match cur {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string().trim_matches('"').to_string(),
    }
}

fn photos(card: &Value) -> String {
    let Some(arr) = card.get("photos").and_then(|p| p.as_array()) else {
        return String::new();
    };
    arr.iter()
        .take(30)
        .filter_map(|p| {
            p.get("big")
                .or_else(|| p.get("c246x328"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sizes_join(card: &Value, key: &str, sep: &str) -> String {
    let Some(arr) = card.get("sizes").and_then(|s| s.as_array()) else {
        return String::new();
    };
    arr.iter()
        .map(|s| jstr(s, &[key]))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(sep)
}

fn sizes_skus(card: &Value) -> String {
    let Some(arr) = card.get("sizes").and_then(|s| s.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|s| s.get("skus").and_then(|x| x.as_array()))
        .flat_map(|a| a.iter())
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join(";")
}
