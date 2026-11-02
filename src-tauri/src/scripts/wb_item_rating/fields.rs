use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("current_period_start", false),
        field("current_period_end", false),
        field("past_period_start", false),
        field("past_period_end", false),
        field("nm_id", true),
        field("title", true),
        field("vendor_code", true),
        field("subject_id", false),
        field("subject_name", false),
        field("brand_name", false),
        field("rating", true),
        field("feedback_rating_current", true),
        field("feedback_rating_dynamics", false),
        field("feedback_count_current", true),
        field("feedback_count_dynamics", false),
        field("five_star_current", false),
        field("four_star_current", false),
        field("three_star_current", false),
        field("two_star_current", false),
        field("one_star_current", false),
        field("disqualified", false),
        field("is_shadowed", false),
        field("seller_rating_current", false),
        field("feedback_increase_current", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.item_rating.{name}")),
        selected,
    }
}

pub struct PeriodMeta<'a> {
    pub cabinet: &'a str,
    pub current_start: &'a str,
    pub current_end: &'a str,
    pub past_start: &'a str,
    pub past_end: &'a str,
}

pub fn build_rows(
    meta: &PeriodMeta<'_>,
    data: &Value,
    fields: &[ScriptField],
) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let items = data
        .get("items")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(items.len() + 1);
    rows.push(selected.iter().map(|f| f.title.clone()).collect());
    for card in &items {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), meta, data, card));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, meta: &PeriodMeta<'_>, data: &Value, card: &Value) -> String {
    match name {
        "cabinet" => meta.cabinet.to_string(),
        "current_period_start" => meta.current_start.to_string(),
        "current_period_end" => meta.current_end.to_string(),
        "past_period_start" => meta.past_start.to_string(),
        "past_period_end" => meta.past_end.to_string(),
        "seller_rating_current" => dig(data, "sellerRating.current"),
        "seller_rating_dynamics" => dig(data, "sellerRating.dynamics"),
        "feedback_increase_current" => dig(data, "feedbackIncrease.current"),
        "feedback_increase_total" => dig(data, "feedbackIncrease.total"),
        "feedback_increase_dynamics" => dig(data, "feedbackIncrease.dynamics"),
        "nm_id" => dig(card, "nmId"),
        "title" => dig(card, "title"),
        "vendor_code" => dig(card, "vendorCode"),
        "subject_id" => dig(card, "subjectId"),
        "subject_name" => dig(card, "subjectName"),
        "brand_name" => dig(card, "brandName"),
        "tag_name" => dig(card, "tagName"),
        "tag_id" => dig(card, "tagId"),
        "pinned_feedback" => dig(card, "pinnedFeedback"),
        "rating" => dig(card, "rating"),
        "feedback_rating_current" => dig(card, "feedbackRating.current"),
        "feedback_rating_dynamics" => dig(card, "feedbackRating.dynamics"),
        "feedback_rating_percentile" => dig(card, "feedbackRating.percentile"),
        "feedback_count_current" => dig(card, "feedbackCount.current"),
        "feedback_count_dynamics" => dig(card, "feedbackCount.dynamics"),
        "five_star_current" => dig(card, "fiveStar.current"),
        "five_star_dynamics" => dig(card, "fiveStar.dynamics"),
        "four_star_current" => dig(card, "fourStar.current"),
        "four_star_dynamics" => dig(card, "fourStar.dynamics"),
        "three_star_current" => dig(card, "threeStar.current"),
        "three_star_dynamics" => dig(card, "threeStar.dynamics"),
        "two_star_current" => dig(card, "twoStar.current"),
        "two_star_dynamics" => dig(card, "twoStar.dynamics"),
        "one_star_current" => dig(card, "oneStar.current"),
        "one_star_dynamics" => dig(card, "oneStar.dynamics"),
        "disqualified" => dig(card, "disqualified"),
        "is_shadowed" => dig(card, "isShadowed"),
        _ => String::new(),
    }
}

fn dig(v: &Value, dotted: &str) -> String {
    let mut cur = v;
    for part in dotted.split('.') {
        cur = match cur.get(part) {
            Some(x) => x,
            None => return String::new(),
        };
    }
    match cur {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}
