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

//! `symbols_check` — окраска EN/RU в диапазоне Google Sheets (без WB API).

use crate::core::google::sheets_symbols;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;

pub struct SymbolsCheckScript;

impl Script for SymbolsCheckScript {
    fn key(&self) -> &'static str {
        "symbols_check"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.symbols_check")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Проверка символов»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let range = ctx
        .params
        .get("symbols_check_range")
        .map(|s| s.as_str())
        .or_else(|| ctx.params.get("cabinets_source_range").map(|s| s.as_str()))
        .unwrap_or("")
        .trim();
    if range.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазон проверки символов (A1)".into(),
        });
    }
    ctx.log.info(format!(
        "Диапазон: {spreadsheet_id}/{sheet_name}!{range}. EN — красный, RU — синий."
    ));
    let client = common::google_sheets_client(ctx).await?;
    sheets_symbols::apply_symbols_check_coloring(
        &client,
        &spreadsheet_id,
        &sheet_name,
        range,
        &|msg| ctx.log.info(msg),
    )
    .await?;
    Ok(ScriptOutput {
        rows_written: 0,
        message: "Проверка символов завершена".into(),
    })
}
