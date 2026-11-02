//! `cards_move_nm` — разъединение карточек (1 nmID за запрос).

use crate::core::rate_profiles::cards_move_nm as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://content-api.wildberries.ru/content/v2/cards/moveNm";

pub struct CardsMoveNmScript;

impl Script for CardsMoveNmScript {
    fn key(&self) -> &'static str {
        "cards_move_nm"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cards_move_nm")
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
    ctx.log.info("Скрипт «Разъединение карточек»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let cab_range = param(ctx, "cabinets_source_range");
    let nm_range = param(ctx, "nm_id_source_range");
    if cab_range.is_empty() || nm_range.is_empty() {
        return Err("Заполните диапазоны: кабинеты и nmID.".into());
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
    let max = cabs.len().max(nms.len());
    if max == 0 {
        return Err("В указанных диапазонах нет данных".into());
    }

    let mut by_cab: HashMap<String, Vec<i64>> = HashMap::new();
    let mut skipped = 0usize;
    for i in 0..max {
        let cabinet = cabs.get(i).map(|s| s.trim()).unwrap_or("");
        let nm_raw = nms.get(i).map(|s| s.as_str()).unwrap_or("");
        if cabinet.is_empty() && nm_raw.trim().is_empty() {
            skipped += 1;
            continue;
        }
        if cabinet.is_empty() {
            return Err(format!(
                "Пустой кабинет в строке {}. Диапазоны должны быть выровнены.",
                i + 1
            )
            .into());
        }
        let Some(nm) = sheet_ranges::parse_i64_cell(nm_raw).filter(|n| *n > 0) else {
            return Err(format!("Некорректный nmID '{nm_raw}' в строке {}", i + 1).into());
        };
        by_cab
            .entry(cabinet.to_string())
            .or_default()
            .push(nm);
    }
    for list in by_cab.values_mut() {
        list.sort_unstable();
        list.dedup();
    }
    let planned: usize = by_cab.values().map(|v| v.len()).sum();
    ctx.log.info(format!(
        "Прочитано артикулов: {planned} (пропущено пустых: {skipped})"
    ));
    if planned == 0 {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет данных".into(),
        });
    }

    let tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut ok = 0usize;
    let mut err = 0usize;
    let mut done = 0usize;

    for (cabinet, nm_ids) in &by_cab {
        let token = common::resolve_wb_token_with_map(ctx, &tokens, cabinet);
        if token.trim().is_empty() {
            return Err(format!("API ключ для кабинета '{cabinet}' не найден").into());
        }
        ctx.log.info(format!(
            "Кабинет {cabinet}: артикулов к разъединению: {}",
            nm_ids.len()
        ));
        for nm in nm_ids {
            ctx.check_cancelled().map_err(|e| e.to_string())?;
            done += 1;
            ctx.log.info(format!(
                "Кабинет {cabinet}: артикул {nm} — запрос {done} из {planned}"
            ));
            let body = serde_json::json!({ "nmIDs": [*nm] });
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
                        err += 1;
                        let t = v
                            .get("errorText")
                            .and_then(|x| x.as_str())
                            .unwrap_or("error");
                        ctx.log.warn(format!("Кабинет {cabinet}: nm={nm}: {t}"));
                    } else {
                        ok += 1;
                    }
                }
                Err(e) => {
                    err += 1;
                    ctx.log.warn(format!("Кабинет {cabinet}: nm={nm}: {e}"));
                }
            }
        }
    }

    Ok(ScriptOutput {
        rows_written: ok,
        message: format!("Успешно: {ok}, ошибок: {err}"),
    })
}

fn param(ctx: &ScriptContext, key: &str) -> String {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
