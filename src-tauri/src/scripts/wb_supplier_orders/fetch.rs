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

//! GET statistics-api `/api/v1/supplier/orders` — пагинация flag=0 по lastChangeDate.

use crate::core::rate_profiles::wb_supplier_orders as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::ScriptContext;
use chrono::NaiveDate;
use serde_json::Value;
use std::collections::HashSet;

const ENDPOINT: &str = "https://statistics-api.wildberries.ru/api/v1/supplier/orders";
/// Statistics API flag=0: max rows per response — fewer means no next page.
const PAGE_LIMIT: usize = 80_000;

/// Все страницы кабинета; дедуп по srid. Возвращает (заказы, число дублей).
pub async fn fetch_all_unique(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    date_from: NaiveDate,
) -> Result<(Vec<Value>, usize), String> {
    let mut cursor = format!("{}T00:00:00", date_from.format("%Y-%m-%d"));
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut duplicates = 0usize;
    let mut page = 0u32;

    loop {
        page += 1;
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let batch = fetch_page(ctx, cabinet, token, &cursor).await?;
        ctx.log.info(format!(
            "Кабинет {cabinet}: страница {page}, dateFrom={cursor}, строк={}",
            batch.len()
        ));
        if batch.is_empty() {
            break;
        }

        let next_cursor = batch
            .last()
            .and_then(|x| x.get("lastChangeDate"))
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let batch_len = batch.len();
        for order in batch {
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

        // Full page only → more data may exist; partial page is the last.
        if batch_len < PAGE_LIMIT {
            break;
        }

        let Some(next) = next_cursor else {
            break;
        };
        if next == cursor {
            ctx.log.warn(format!(
                "Кабинет {cabinet}: lastChangeDate не сдвинулся ({next}) — стоп"
            ));
            break;
        }
        cursor = next;
    }

    Ok((out, duplicates))
}

async fn fetch_page(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    date_from: &str,
) -> Result<Vec<Value>, String> {
    let encoded = urlencoding_date(date_from);
    let url = format!("{ENDPOINT}?dateFrom={encoded}&flag=0");
    let profile = rate::profile();
    let auth = token.trim().strip_prefix("Bearer ").unwrap_or(token).trim();
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            &url,
            Auth::Raw(auth),
        )
        .await?;

    Ok(match v {
        Value::Array(a) => a,
        other => other.as_array().cloned().unwrap_or_default(),
    })
}

/// Минимальный encode для dateFrom (RFC3339 / ISO без пробелов обычно).
fn urlencoding_date(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | ':' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}
