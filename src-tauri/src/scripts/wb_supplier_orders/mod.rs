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

//! Заказы WB Statistics API (`wb_supplier_orders`).

pub mod fetch;
pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, dates};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use self::fields::{build_rows, default_fields, normalize_fields, price_fields_empty_diag};
use self::fetch::fetch_all_unique;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const MAX_RETENTION_DAYS: i64 = 90;

pub struct WbSupplierOrdersScript;

impl Script for WbSupplierOrdersScript {
    fn key(&self) -> &'static str {
        "wb_supplier_orders"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_supplier_orders")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Заказы» (supplier/orders): старт");
    ctx.check_cancelled()?;

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (period_from, period_to) = dates::resolve_period(ctx, 14)?;
    let mut request_from = period_from;
    let min_from = period_to - Duration::days(MAX_RETENTION_DAYS - 1);
    if request_from < min_from {
        ctx.log.info(format!(
            "Период {period_from}..{period_to} длиннее {MAX_RETENTION_DAYS} дн. API хранит {MAX_RETENTION_DAYS} дн. — запрос с {min_from}"
        ));
        request_from = min_from;
    } else {
        ctx.log.info(format!(
            "Период: {period_from}..{period_to} (запрос API dateFrom −1д)"
        ));
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = normalize_fields(&common::resolve_fields(ctx, default_fields()));
    if fields.iter().filter(|f| f.selected).count() == 0 {
        return Err("Не выбраны поля для выгрузки.".into());
    }

    let wb_tokens = common::load_wb_tokens()?;
    // All selected cabinets in parallel (per-cabinet rate limit, not global CAP=3).
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::wb_cabinet_parallelism(cabinets.len()),
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, None));
                }
                // API flag=0 идёт по lastChangeDate — берём dateFrom на сутки раньше;
                // в таблицу попадают только заказы с date в [period_from..period_to].
                let api_from = request_from - Duration::days(1);
                match fetch_cabinet(
                    ctx,
                    &cabinet,
                    token.trim(),
                    api_from,
                    period_from,
                    period_to,
                )
                .await
                {
                    Ok(data) => Ok((cabinet, Some(data))),
                    Err(e) => {
                        ctx.log.warn(e);
                        Ok((cabinet, None))
                    }
                }
            }
        },
    )
    .await?;

    ctx.check_cancelled()?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    let mut price_diag_done = false;
    for (cabinet, data) in parts {
        let Some(orders) = data else {
            continue;
        };
        if !price_diag_done {
            if let Some(msg) = price_fields_empty_diag(&orders, &fields) {
                ctx.log.info(msg);
            }
            price_diag_done = true;
        }
        total += orders.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, &orders, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} заказов на лист «{sheet_name}»"),
    })
}

async fn fetch_cabinet(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    api_from: NaiveDate,
    filter_from: NaiveDate,
    filter_to: NaiveDate,
) -> Result<Vec<Value>, String> {
    ctx.log.info(format!(
        "Кабинет {cabinet}: запрос dateFrom={api_from}, фильтр date заказа {filter_from}..{filter_to}"
    ));
    let (orders, duplicates) = fetch_all_unique(ctx, cabinet, token, api_from).await?;
    let filtered: Vec<Value> = orders
        .into_iter()
        .filter(|o| order_date_in_range(o, filter_from, filter_to))
        .collect();
    ctx.log.info(format!(
        "Кабинет {cabinet}: после фильтра={}, дублей при дедупе={duplicates}",
        filtered.len()
    ));
    Ok(filtered)
}

fn order_date_in_range(o: &Value, from: NaiveDate, to: NaiveDate) -> bool {
    let Some(raw) = o.get("date").and_then(|x| x.as_str()).map(str::trim) else {
        return false;
    };
    if raw.is_empty() {
        return false;
    }
    let Some(d) = parse_order_date(raw) else {
        return false;
    };
    d >= from && d <= to
}

fn parse_order_date(raw: &str) -> Option<NaiveDate> {
    let s = raw.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_local().date());
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(ndt.date());
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(ndt.date());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
