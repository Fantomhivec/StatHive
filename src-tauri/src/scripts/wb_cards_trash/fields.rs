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

//! DefaultFields корзины WB — schema /content/v2/get/cards/trash + cabinet.

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

fn f(name: &str, title_key: &str, selected: bool) -> ScriptField {
    ScriptField::new(name, crate::core::i18n::t(title_key), selected)
}

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        f("cabinet", "fields.cards_trash.cabinet", true),
        f("nmID", "fields.cards_trash.nmID", true),
        f("vendorCode", "fields.cards_trash.vendorCode", true),
        f("kizMarked", "fields.cards_trash.kizMarked", false),
        f("subjectID", "fields.cards_trash.subjectID", false),
        f("subjectName", "fields.cards_trash.subjectName", true),
        f("photos.big", "fields.cards_trash.photos.big", false),
        f("photos.c246x328", "fields.cards_trash.photos.c246x328", false),
        f("photos.c516x688", "fields.cards_trash.photos.c516x688", false),
        f("photos.square", "fields.cards_trash.photos.square", false),
        f("photos.tm", "fields.cards_trash.photos.tm", false),
        f("video", "fields.cards_trash.video", false),
        f("wholesale.enabled", "fields.cards_trash.wholesale.enabled", false),
        f("wholesale.quantum", "fields.cards_trash.wholesale.quantum", false),
        f("dimensions.length", "fields.cards_trash.dimensions.length", false),
        f("dimensions.width", "fields.cards_trash.dimensions.width", false),
        f("dimensions.height", "fields.cards_trash.dimensions.height", false),
        f(
            "dimensions.weightBrutto",
            "fields.cards_trash.dimensions.weightBrutto",
            false,
        ),
        f(
            "dimensions.isValid",
            "fields.cards_trash.dimensions.isValid",
            false,
        ),
        f("characteristics", "fields.cards_trash.characteristics", false),
        f("sizes.chrtID", "fields.cards_trash.sizes.chrtID", false),
        f("sizes.techSize", "fields.cards_trash.sizes.techSize", false),
        f("sizes.wbSize", "fields.cards_trash.sizes.wbSize", false),
        f("sizes.skus", "fields.cards_trash.sizes.skus", false),
        f("createdAt", "fields.cards_trash.createdAt", true),
        f("trashedAt", "fields.cards_trash.trashedAt", true),
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
        "nmID" => json_str(card, "nmID"),
        "vendorCode" => json_str(card, "vendorCode"),
        "kizMarked" => json_str(card, "kizMarked"),
        "subjectID" => json_str(card, "subjectID"),
        "subjectName" => json_str(card, "subjectName"),
        "photos.big" => photos_join(card, "big"),
        "photos.c246x328" => photos_join(card, "c246x328"),
        "photos.c516x688" => photos_join(card, "c516x688"),
        "photos.square" => photos_join(card, "square"),
        "photos.tm" => photos_join(card, "tm"),
        "video" => json_str(card, "video"),
        "wholesale.enabled" => path_str(card, &["wholesale", "enabled"]),
        "wholesale.quantum" => path_str(card, &["wholesale", "quantum"]),
        "dimensions.length" => path_str(card, &["dimensions", "length"]),
        "dimensions.width" => path_str(card, &["dimensions", "width"]),
        "dimensions.height" => path_str(card, &["dimensions", "height"]),
        "dimensions.weightBrutto" => path_str(card, &["dimensions", "weightBrutto"]),
        "dimensions.isValid" => path_str(card, &["dimensions", "isValid"]),
        "characteristics" => characteristics_cell(card),
        "sizes.chrtID" => sizes_join(card, "chrtID"),
        "sizes.techSize" => sizes_join(card, "techSize"),
        "sizes.wbSize" => sizes_join(card, "wbSize"),
        "sizes.skus" => sizes_skus(card),
        "createdAt" => sheet_dates::format_wb_timestamp(&json_str(card, "createdAt")),
        "trashedAt" => sheet_dates::format_wb_timestamp(&json_str(card, "trashedAt")),
        _ => String::new(),
    }
}

fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
    }
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

fn photos_join(card: &Value, key: &str) -> String {
    let Some(arr) = card.get("photos").and_then(|p| p.as_array()) else {
        return String::new();
    };
    arr.iter()
        .filter_map(|p| p.get(key).and_then(|x| x.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join(";")
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

fn sizes_join(card: &Value, key: &str) -> String {
    let Some(arr) = card.get("sizes").and_then(|s| s.as_array()) else {
        return String::new();
    };
    arr.iter()
        .map(|s| json_str(s, key))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(";")
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_kiz_and_dimensions_is_valid() {
        let card = json!({
            "kizMarked": true,
            "isValid": true,
            "dimensions": {
                "length": 55,
                "isValid": false
            }
        });
        assert_eq!(cell("cab", &card, "kizMarked"), "true");
        assert_eq!(cell("cab", &card, "dimensions.isValid"), "false");
        assert!(cell("cab", &card, "isValid").is_empty());
    }

    #[test]
    fn characteristics_include_id() {
        let card = json!({
            "characteristics": [
                { "id": 14177449, "name": "Цвет", "value": ["красно-сиреневый"] }
            ]
        });
        let s = cell("c", &card, "characteristics");
        assert!(s.contains("14177449:Цвет:красно-сиреневый"));
    }

    #[test]
    fn photo_columns_use_exact_size_keys() {
        let card = json!({
            "photos": [
                {
                    "big": "https://example/big.webp",
                    "c246x328": "https://example/246.webp",
                    "c516x688": "https://example/516.webp",
                    "square": "https://example/sq.webp",
                    "tm": "https://example/tm.webp"
                },
                {
                    "big": "https://example/big2.webp",
                    "c246x328": "https://example/246b.webp"
                }
            ]
        });
        assert_eq!(
            cell("c", &card, "photos.big"),
            "https://example/big.webp;https://example/big2.webp"
        );
        assert_eq!(
            cell("c", &card, "photos.c246x328"),
            "https://example/246.webp;https://example/246b.webp"
        );
        assert_eq!(cell("c", &card, "photos.c516x688"), "https://example/516.webp");
        assert_eq!(cell("c", &card, "photos.square"), "https://example/sq.webp");
        assert_eq!(cell("c", &card, "photos.tm"), "https://example/tm.webp");
    }
}
