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

//! `content_media_save` — загрузка медиа по ссылкам (без записи в Sheets).

use crate::core::rate_profiles::content_media_save as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://content-api.wildberries.ru/content/v3/media/save";
/// Docs: up to 30 images + 1 video in `data`.
const MAX_LINKS: usize = 31;

pub struct ContentMediaSaveScript;

impl Script for ContentMediaSaveScript {
    fn key(&self) -> &'static str {
        "content_media_save"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.content_media_save")
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
    ctx.log.info("Скрипт «Загрузка медиа»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let cab_range = param(ctx, "cabinets_source_range");
    let nm_range = param(ctx, "nm_id_source_range");
    let links_range = param(ctx, "content_media_links_source_range");
    if cab_range.is_empty() || nm_range.is_empty() || links_range.is_empty() {
        return Err(
            "Укажите таблицу, лист, диапазоны кабинетов, артикулов (nmID) и ссылок на медиа."
                .into(),
        );
    }
    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(
        &client,
        &spreadsheet_id,
        &sheet_name,
        &[cab_range.as_str(), nm_range.as_str()],
    )
    .await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    let links_matrix =
        sheet_ranges::read_matrix(&client, &spreadsheet_id, &sheet_name, &links_range).await?;
    let max = cabs
        .len()
        .max(nms.len())
        .max(links_matrix.len());
    if max == 0 {
        ctx.log.info("В указанных диапазонах нет данных.");
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет данных".into(),
        });
    }

    let tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut ok = 0usize;
    let mut errors = 0usize;

    for i in 0..max {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let cabinet = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let nm_raw = nms.get(i).map(|s| s.as_str()).unwrap_or("");
        if cabinet.is_empty() || nm_raw.trim().is_empty() {
            continue;
        }
        let Some(nm) = sheet_ranges::parse_i64_cell(nm_raw).filter(|n| *n > 0) else {
            ctx.log.info(format!(
                "Строка {}: пропуск — некорректный nmID '{nm_raw}'.",
                i + 1
            ));
            continue;
        };
        let mut links = parse_links_row(links_matrix.get(i));
        if links.is_empty() {
            ctx.log.info(format!(
                "Строка {} (nmID={nm}): нет ссылок — пропуск.",
                i + 1
            ));
            continue;
        }
        if links.len() > MAX_LINKS {
            ctx.log.info(format!(
                "Строка {} (nmID={nm}): ссылок {}, берём первые {MAX_LINKS}.",
                i + 1,
                links.len()
            ));
            links.truncate(MAX_LINKS);
        }
        let token = common::resolve_wb_token_with_map(ctx, &tokens, cabinet);
        if token.trim().is_empty() {
            ctx.log.info(format!(
                "Строка {}: кабинет '{cabinet}' без API ключа — пропуск.",
                i + 1
            ));
            errors += 1;
            continue;
        }
        let body = serde_json::json!({
            "nmId": nm,
            "data": links
        });
        match ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                URL,
                Auth::Bearer(token.trim()),
                &body,
            )
            .await
        {
            Ok(v) => {
                if v.get("error").and_then(|e| e.as_bool()).unwrap_or(false) {
                    errors += 1;
                    let t = v
                        .get("errorText")
                        .and_then(|x| x.as_str())
                        .unwrap_or("error");
                    let extra = v
                        .get("additionalErrors")
                        .filter(|a| !a.is_null())
                        .map(|a| format!("; additionalErrors={a}"))
                        .unwrap_or_default();
                    ctx.log.warn(format!("nmID={nm}: {t}{extra}"));
                } else {
                    ok += 1;
                }
            }
            Err(e) => {
                errors += 1;
                ctx.log.warn(format!("nmID={nm}: {e}"));
            }
        }
    }

    ctx.log
        .info(format!("Готово. Успешно: {ok}, ошибок: {errors}."));
    Ok(ScriptOutput {
        rows_written: ok,
        message: format!("Успешно: {ok}, ошибок: {errors}"),
    })
}

fn parse_links_row(row: Option<&Vec<String>>) -> Vec<String> {
    let Some(row) = row else {
        return Vec::new();
    };
    if row.is_empty() {
        return Vec::new();
    }
    if row.len() == 1 {
        return row[0]
            .split(|c| matches!(c, ',' | ';' | '\n' | '\r' | '\t' | '|'))
            .map(|s| s.trim().to_string())
            .filter(|s| is_url(s))
            .collect();
    }
    row.iter()
        .map(|s| s.trim().to_string())
        .filter(|s| is_url(s))
        .collect()
}

fn is_url(s: &str) -> bool {
    !s.is_empty() && s.to_ascii_lowercase().contains("http")
}

fn param(ctx: &ScriptContext, key: &str) -> String {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
