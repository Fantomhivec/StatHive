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

pub mod fields;

use crate::core::rate_profiles::prices_export as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_rows, default_fields};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const PRICES_URL: &str = "https://discounts-prices-api.wildberries.ru/api/v2/list/goods/filter";
const PAGE_LIMIT: i64 = 1000;

/// Выгрузка цен WB (`prices_export`) → Google Sheets.
pub struct PricesScript;

impl Script for PricesScript {
    fn key(&self) -> &'static str {
        "prices_export"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.prices_export")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_prices(ctx).await })
    }
}

async fn run_prices(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Выгрузка цен»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log
        .info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::wb_cabinet_parallelism(cabinets.len()),
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::new()));
                }
                let goods = fetch_all_prices(ctx, &cabinet, token.trim()).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} позиций", goods.len()));
                Ok((cabinet, goods))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, goods) in parts {
        if goods.is_empty() {
            continue;
        }
        total += goods.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, &goods, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} позиций на лист «{sheet_name}»"),
    })
}

async fn fetch_all_prices(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut offset: i64 = 0;
    let mut page = 0u32;
    let profile = rate::profile();

    loop {
        page += 1;
        let url = format!("{PRICES_URL}?limit={PAGE_LIMIT}&offset={offset}");
        let v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                &url,
                Auth::Bearer(token),
            )
            .await?;

        let goods = v
            .pointer("/data/listGoods")
            .and_then(|g| g.as_array())
            .cloned()
            .unwrap_or_default();
        let n = goods.len();
        ctx.log
            .info(format!("Страница {page}: {n} позиций (offset={offset})"));
        if n == 0 {
            break;
        }
        if v.get("error").and_then(|e| e.as_bool()).unwrap_or(false) {
            let text = v
                .get("errorText")
                .and_then(|t| t.as_str())
                .unwrap_or("error=true");
            return Err(format!("[{cabinet}] list/goods/filter: {text}"));
        }
        all.extend(goods);
        offset += n as i64;
        if (n as i64) < PAGE_LIMIT {
            break;
        }
    }

    Ok(all)
}
