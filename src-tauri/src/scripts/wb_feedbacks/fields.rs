//! Поля архивных отзывов (паритет FeedbacksArchiveScript).

use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use serde_json::Value;

const SPECS: &[(&str, bool)] = &[
    ("cabinet", true),
    ("id", true),
    ("text", true),
    ("pros", true),
    ("cons", true),
    ("productValuation", true),
    ("createdDate", true),
    ("answerText", true),
    ("answerState", true),
    ("editable", true),
    ("state", true),
    ("nmId", true),
    ("imtId", false),
    ("productName", true),
    ("supplierArticle", false),
    ("supplierName", false),
    ("brandName", true),
    ("size", true),
    ("userName", true),
    ("orderStatus", true),
    ("matchingSize", false),
    ("wasViewed", false),
    ("lastOrderCreatedAt", false),
    ("color", false),
    ("subjectId", false),
    ("subjectName", false),
    ("parentFeedbackId", false),
    ("childFeedbackId", false),
    ("videoPreviewImage", true),
    ("videoLink", true),
    ("videoDurationSec", true),
    ("photoLinks", true),
    ("error", false),
    ("errorText", false),
];

pub fn default_fields() -> Vec<ScriptField> {
    SPECS
        .iter()
        .map(|(name, selected)| ScriptField {
            name: (*name).into(),
            title: crate::core::i18n::t(&format!("fields.feedbacks_archive.{name}")),
            selected: *selected,
        })
        .collect()
}

pub fn cell(name: &str, fb: &Value, cabinet: &str) -> String {
    if name == "cabinet" {
        return cabinet.to_string();
    }
    if name == "videoLink" {
        return video_links(fb);
    }
    if name == "orderStatus" {
        return order_status(token(name, fb));
    }
    if name == "matchingSize" {
        return matching_size(token(name, fb));
    }
    if name == "createdDate" || name == "lastOrderCreatedAt" {
        let raw = scalar(token(name, fb));
        return if raw.is_empty() {
            String::new()
        } else {
            sheet_dates::format_wb_timestamp(&raw)
        };
    }
    match token(name, fb) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Bool(true)) => "Да".into(),
        Some(Value::Bool(false)) => "Нет".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
    }
}

pub fn extract_photos(fb: &Value) -> Vec<String> {
    fb.get("photoLinks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.get("fullSize").and_then(|x| x.as_str()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn token<'a>(name: &str, fb: &'a Value) -> Option<&'a Value> {
    let details = |key: &str| nested(fb, "productDetails", key);
    match name {
        "id" => fb.get("id"),
        "text" => fb.get("text"),
        "pros" => fb.get("pros"),
        "cons" => fb.get("cons"),
        "productValuation" => fb.get("productValuation"),
        "createdDate" => fb.get("createdDate"),
        "answerText" => nested(fb, "answer", "text"),
        "answerState" => nested(fb, "answer", "state"),
        "editable" => fb.get("editable"),
        "state" => fb.get("state"),
        "error" => fb.get("error"),
        "errorText" => fb.get("errorText"),
        "nmId" => details("nmId"),
        "imtId" => details("imtId"),
        "productName" => details("productName"),
        "supplierArticle" => details("supplierArticle"),
        "supplierName" => details("supplierName"),
        "brandName" => details("brandName"),
        "size" => details("size"),
        "userName" => present(details("userName")).or(present(fb.get("userName"))),
        "orderStatus" => present(details("orderStatus")).or(present(fb.get("orderStatus"))),
        "matchingSize" => present(details("matchingSize")).or(present(fb.get("matchingSize"))),
        "wasViewed" => present(details("wasViewed")).or(present(fb.get("wasViewed"))),
        "lastOrderCreatedAt" => {
            present(details("lastOrderCreatedAt")).or(present(fb.get("lastOrderCreatedAt")))
        }
        "color" => present(details("color")).or(present(fb.get("color"))),
        "subjectId" => present(details("subjectId")).or(present(fb.get("subjectId"))),
        "subjectName" => present(details("subjectName")).or(present(fb.get("subjectName"))),
        "parentFeedbackId" => fb.get("parentFeedbackId"),
        "childFeedbackId" => fb.get("childFeedbackId"),
        "videoPreviewImage" => nested(fb, "video", "previewImage"),
        "videoDurationSec" => nested(fb, "video", "durationSec"),
        _ => None,
    }
}

fn nested<'a>(v: &'a Value, a: &str, b: &str) -> Option<&'a Value> {
    let first = v.get(a)?;
    if !first.is_object() {
        return None;
    }
    first.get(b)
}

fn present(v: Option<&Value>) -> Option<&Value> {
    v.filter(|x| !x.is_null())
}

fn scalar(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

fn video_links(fb: &Value) -> String {
    let Some(video) = fb.get("video") else {
        return String::new();
    };
    if video.is_null() {
        return String::new();
    }
    if let Some(arr) = video.as_array() {
        return arr
            .iter()
            .filter_map(|item| item.get("link").and_then(|x| x.as_str()))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
    }
    video
        .get("link")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn order_status(v: Option<&Value>) -> String {
    let raw = scalar(v);
    if raw.is_empty() {
        return String::new();
    }
    match raw.trim().to_ascii_lowercase().as_str() {
        "buyout" => "Выкуплен".into(),
        "rejected" => "Отказались".into(),
        "returned" => "Возврат".into(),
        "notspecified" => "Статус не присвоен".into(),
        _ => raw,
    }
}

fn matching_size(v: Option<&Value>) -> String {
    let raw = scalar(v);
    let key = raw.trim();
    if key.is_empty() || is_dash_placeholder(key) {
        return "Безразмерный товар".into();
    }
    match key.to_ascii_lowercase().as_str() {
        "ok" => "Соответствует размеру".into(),
        "smaller" => "Маломерит".into(),
        "bigger" => "Большемерит".into(),
        _ => key.to_string(),
    }
}

fn is_dash_placeholder(trimmed: &str) -> bool {
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| matches!(c, '-' | '\u{2013}' | '\u{2014}' | ' ' | '\u{00a0}'))
}
