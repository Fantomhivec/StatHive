//! `cards_move_to_trash` — перенос карточек в корзину (без записи в Sheets).

use crate::core::rate_profiles::cards_move_to_trash as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://content-api.wildberries.ru/content/v2/cards/delete/trash";
const BATCH: usize = 1000;

pub struct CardsMoveToTrashScript;

impl Script for CardsMoveToTrashScript {
    fn key(&self) -> &'static str {
        "cards_move_to_trash"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cards_move_to_trash")
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
    ctx.log.info("Скрипт «Перенести в корзину»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let nm_range = param(ctx, "nm_id_source_range");
    let cab_range = param(ctx, "cabinets_source_range");
    if nm_range.is_empty() || cab_range.is_empty() {
        return Err(
            "Укажите ID таблицы, лист, диапазон артикулов и диапазон кабинетов для переноса"
                .into(),
        );
    }
    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(
        &client,
        &spreadsheet_id,
        &sheet_name,
        &[nm_range.as_str(), cab_range.as_str()],
    )
    .await?;
    let nm_vals = cols.first().cloned().unwrap_or_default();
    let cab_vals = cols.get(1).cloned().unwrap_or_default();
    let max = nm_vals.len().max(cab_vals.len());
    if max == 0 {
        ctx.log.info("Диапазон артикулов пуст — нечего переносить.");
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет данных".into(),
        });
    }

    let mut per_cab: HashMap<String, Vec<i64>> = HashMap::new();
    let mut display: HashMap<String, String> = HashMap::new();
    let mut invalid = 0usize;
    let mut missing_cab = 0usize;

    for i in 0..max {
        let nm_raw = nm_vals.get(i).map(|s| s.as_str()).unwrap_or("");
        if nm_raw.trim().is_empty() {
            continue;
        }
        let Some(nm) = sheet_ranges::parse_i64_cell(nm_raw).filter(|n| *n > 0) else {
            invalid += 1;
            continue;
        };
        let cab_raw = cab_vals.get(i).map(|s| s.as_str()).unwrap_or("").trim();
        if cab_raw.is_empty() {
            missing_cab += 1;
            continue;
        }
        let key = sheet_ranges::normalize_cabinet_name(cab_raw).to_ascii_lowercase();
        display.entry(key.clone()).or_insert_with(|| cab_raw.to_string());
        per_cab.entry(key).or_default().push(nm);
    }
    if invalid > 0 {
        ctx.log.warn(format!("Некорректные артикулы: {invalid}"));
    }
    if missing_cab > 0 {
        ctx.log
            .warn(format!("Артикулы без кабинета: {missing_cab}"));
    }
    if per_cab.is_empty() {
        ctx.log.info("Подходящих строк для переноса не найдено.");
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет данных".into(),
        });
    }

    let tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let mut ok_batches = 0usize;

    let mut keys: Vec<_> = per_cab.keys().cloned().collect();
    keys.sort();
    for key in keys {
        let nms = per_cab.get(&key).cloned().unwrap_or_default();
        let label = display.get(&key).cloned().unwrap_or_else(|| key.clone());
        let token = tokens
            .get(&key)
            .cloned()
            .unwrap_or_else(|| common::resolve_wb_token_with_map(ctx, &tokens, &label));
        if token.trim().is_empty() {
            ctx.log
                .warn(format!("[{label}] Пропуск: не найден API ключ."));
            continue;
        }
        ctx.log
            .info(format!("Кабинет {label}: {} артикулов к переносу.", nms.len()));
        for (bi, chunk) in nms.chunks(BATCH).enumerate() {
            ctx.check_cancelled().map_err(|e| e.to_string())?;
            let body = serde_json::json!({ "nmIDs": chunk });
            match ctx
                .request
                .post_json(
                    Limit {
                        cabinet: &label,
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
                    let err = v.get("error").and_then(|e| e.as_bool()).unwrap_or(false);
                    if err {
                        let text = v
                            .get("errorText")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown");
                        ctx.log
                            .warn(format!("[{label}] Батч {}: ошибка {text}", bi + 1));
                    } else {
                        ok_batches += 1;
                        ctx.log.info(format!(
                            "[{label}] Батч {} успешно перенесён ({} шт).",
                            bi + 1,
                            chunk.len()
                        ));
                    }
                }
                Err(e) => ctx.log.warn(format!("[{label}] Батч {}: {e}", bi + 1)),
            }
        }
    }

    ctx.log.info("Перенос карточек в корзину завершён.");
    Ok(ScriptOutput {
        rows_written: ok_batches,
        message: format!("Успешных батчей: {ok_batches}"),
    })
}

fn param(ctx: &ScriptContext, key: &str) -> String {
    ctx.params
        .get(key)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
