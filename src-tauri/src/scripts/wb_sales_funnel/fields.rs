use crate::core::script_field::ScriptField;
use serde_json::Value;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("nmId", true),
        field("title", true),
        field("vendorCode", false),
        field("brandName", false),
        field("subjectId", false),
        field("subjectName", false),
        field("productRating", false),
        field("feedbackRating", false),
        field("stocks.wb", false),
        field("stocks.mp", false),
        field("stocks.balanceSum", false),
        field("selected.period.start", false),
        field("selected.period.end", false),
        field("selected.openCount", true),
        field("selected.cartCount", true),
        field("selected.orderCount", true),
        field("selected.orderSum", false),
        field("selected.buyoutCount", true),
        field("selected.buyoutSum", false),
        field("selected.cancelCount", false),
        field("selected.cancelSum", false),
        field("selected.avgPrice", false),
        field("selected.avgOrdersCountPerDay", false),
        field("selected.shareOrderPercent", false),
        field("selected.addToWishlist", false),
        field("selected.timeToReady.days", false),
        field("selected.timeToReady.hours", false),
        field("selected.timeToReady.mins", false),
        field("selected.localizationPercent", false),
        field("selected.wbClub.orderCount", false),
        field("selected.wbClub.orderSum", false),
        field("selected.wbClub.buyoutSum", false),
        field("selected.wbClub.buyoutCount", false),
        field("selected.wbClub.cancelSum", false),
        field("selected.wbClub.cancelCount", false),
        field("selected.wbClub.avgPrice", false),
        field("selected.wbClub.buyoutPercent", false),
        field("selected.wbClub.avgOrderCountPerDay", false),
        field("selected.conversions.addToCartPercent", false),
        field("selected.conversions.cartToOrderPercent", false),
        field("selected.conversions.buyoutPercent", false),
        field("past.openCount", false),
        field("past.cartCount", false),
        field("past.orderCount", false),
        field("past.orderSum", false),
        field("past.buyoutCount", false),
        field("past.buyoutSum", false),
        field("past.cancelCount", false),
        field("past.cancelSum", false),
        field("comparison.openCountDynamic", false),
        field("comparison.orderCountDynamic", false),
        field("comparison.buyoutCountDynamic", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.sales_funnel_products.{name}")),
        selected,
    }
}

pub fn build_rows(cabinet: &str, products: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = Vec::with_capacity(products.len() + 1);
    rows.push(selected.iter().map(|f| f.title.clone()).collect());
    for rec in products {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), cabinet, rec));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, cabinet: &str, rec: &Value) -> String {
    if name.eq_ignore_ascii_case("cabinet") {
        return cabinet.to_string();
    }
    let path = if name.starts_with("stocks.")
        || (!name.contains('.')
            && !name.starts_with("selected.")
            && !name.starts_with("past.")
            && !name.starts_with("comparison."))
    {
        format!("product.{name}")
    } else {
        format!("statistic.{name}")
    };
    dig(rec, &path)
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
