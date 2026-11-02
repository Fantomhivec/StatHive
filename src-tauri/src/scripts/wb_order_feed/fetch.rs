//! POST order-feed пагинация (паритет OrderFeedScript).

use crate::core::rate_profiles::order_feed as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::ScriptContext;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde_json::{json, Value};
use std::collections::HashSet;

const ENDPOINT: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v1/order-feed";
const PAGE_LIMIT: i64 = 10_000;

pub struct FeedPage {
    pub orders: Vec<Value>,
    pub snapshot_time: String,
    pub currency: String,
}

/// Одна страница order-feed.
pub async fn fetch_page(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    from: NaiveDate,
    to: NaiveDate,
    offset: i64,
    snapshot_time: Option<&str>,
) -> Result<FeedPage, String> {
    ctx.check_cancelled().map_err(|e| e.to_string())?;
    let mut pagination = json!({
        "offset": offset,
        "limit": PAGE_LIMIT
    });
    if offset > 0 {
        if let Some(snap) = snapshot_time.map(str::trim).filter(|s| !s.is_empty()) {
            pagination["snapshotTime"] = json!(to_rfc3339_snapshot(snap));
        }
    }
    let body = json!({
        "selectedPeriod": {
            "start": format_utc_day_start(from),
            "end": format_utc_day_end(to)
        },
        "nmIds": [],
        "subjectIds": [],
        "brandNames": [],
        "tagIds": [],
        "pagination": pagination
    });
    let profile = rate::profile();
    let auth = token.trim().strip_prefix("Bearer ").unwrap_or(token).trim();
    let v = ctx
        .request
        .post_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            ENDPOINT,
            Auth::Raw(auth),
            &body,
        )
        .await?;

    let data = v.get("data").cloned().unwrap_or(Value::Null);
    let orders = data
        .get("orders")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let page_snap = data
        .get("snapshotTime")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let currency = data
        .get("currency")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    Ok(FeedPage {
        orders,
        snapshot_time: page_snap,
        currency,
    })
}

/// Все страницы кабинета; дедуп по srid.
pub async fn fetch_all_unique(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<(Vec<Value>, String, String, usize), String> {
    let mut offset: i64 = 0;
    let mut snapshot: Option<String> = None;
    let mut currency = String::new();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut duplicates = 0usize;

    loop {
        let page = fetch_page(
            ctx,
            cabinet,
            token,
            from,
            to,
            offset,
            snapshot.as_deref(),
        )
        .await?;
        if offset == 0 && !page.snapshot_time.trim().is_empty() {
            snapshot = Some(to_rfc3339_snapshot(&page.snapshot_time));
            ctx.log.info(format!(
                "Кабинет {cabinet}: snapshotTime={} (из ответа: {})",
                snapshot.as_deref().unwrap_or(""),
                page.snapshot_time
            ));
        }
        if !page.currency.is_empty() {
            currency = page.currency;
        }
        let count = page.orders.len() as i64;
        ctx.log.info(format!(
            "Кабинет {cabinet}: offset={offset}, заказов={count}"
        ));

        for order in page.orders {
            let srid = order
                .get("srid")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !srid.is_empty() && !seen.insert(srid) {
                duplicates += 1;
                continue;
            }
            out.push(order);
        }

        if count == 0 || count < PAGE_LIMIT {
            break;
        }
        offset += count;
    }

    Ok((
        out,
        snapshot.unwrap_or_default(),
        currency,
        duplicates,
    ))
}

fn format_utc_day_start(d: NaiveDate) -> String {
    format!("{}T00:00:00.0000000Z", d.format("%Y-%m-%d"))
}

fn format_utc_day_end(d: NaiveDate) -> String {
    // end of day UTC (AddDays(1).AddTicks(-1))
    format!("{}T23:59:59.9999999Z", d.format("%Y-%m-%d"))
}

/// API snapshotTime → RFC3339 UTC для следующей страницы.
pub fn to_rfc3339_snapshot(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt
            .with_timezone(&chrono::Utc)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
    }
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M:%S",
    ];
    for fmt in formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc);
            let _ = dt.time().num_seconds_from_midnight();
            return dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        }
    }
    s.to_string()
}
