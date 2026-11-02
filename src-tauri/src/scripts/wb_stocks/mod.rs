pub mod load;
pub mod parse;

pub use load::{load_filters, CabinetFilter};

use crate::core::rate_profiles::wb_stocks as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::load::{build_data_row, build_header, default_fields};
use self::parse::param;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use tokio::time::sleep;

const STOCKS_URL: &str =
    "https://seller-analytics-api.wildberries.ru/api/analytics/v1/stocks-report/seller-warehouses";
const NM_BATCH: usize = 1000;
const PAGE_LIMIT: i64 = 250_000;

/// Остатки FBS (`wb_stocks`) — nmId/chrtId из листа-источника (Массив) → Sheets.
pub struct WbStocksScript;

impl Script for WbStocksScript {
    fn key(&self) -> &'static str {
        "wb_stocks"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_stocks")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_wb_stocks(ctx).await })
    }
}


async fn run_wb_stocks(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Старт");

    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(_) => {
            return Ok(ScriptOutput {
                rows_written: 0,
                message: "Укажите таблицу и лист назначения".into(),
            });
        }
    };

    let src_id = param(ctx, "source_spreadsheet_id")
        .or_else(|| param(ctx, "second_spreadsheet_id"))
        .unwrap_or_else(|| dest_id.clone());
    let src_sheet = param(ctx, "source_sheet_name")
        .or_else(|| param(ctx, "second_sheet_name"))
        .unwrap_or_else(|| "Массив универсал".into());

    ctx.log.info(format!(
        "Источник: {src_id}, лист: {src_sheet}; назначение: «{dest_sheet}»"
    ));

    let fields = common::resolve_fields(ctx, default_fields());
    let client = common::google_sheets_client(ctx).await?;

    let by_cabinet = load_filters(ctx, &client, src_id.trim(), src_sheet.trim()).await?;
    if by_cabinet.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет строк с кабинетом, nmId и chrtId в источнике".into(),
        });
    }
    ctx.log.info(format!(
        "Кабинетов после фильтрации: {}",
        by_cabinet.len()
    ));

    let wb_tokens = common::load_wb_tokens()?;
    let cab_order: Vec<String> = by_cabinet.keys().cloned().collect();
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let filter = by_cabinet.get(&cabinet).cloned();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                let Some(filter) = filter else {
                    return Ok((cabinet, Vec::new()));
                };
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Кабинет {cabinet}: нет API_KEY, пропуск"));
                    return Ok((cabinet, Vec::new()));
                }
                let items = fetch_stocks(ctx, &cabinet, token.trim(), &filter).await?;
                ctx.log
                    .info(format!("Кабинет {cabinet}: строк={}", items.len()));
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows = vec![build_header(&fields)];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        for it in &items {
            all_rows.push(build_data_row(&cabinet, it, &fields));
            total += 1;
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    ctx.log.info(format!(
        "Кабинеты завершены. Строк к записи: {total}"
    ));
    ctx.log.info(format!(
        "Запись в таблицу {dest_id}, лист {dest_sheet}"
    ));
    let written =
        common::write_sheet_rows_with(ctx, &client, dest_id.trim(), dest_sheet.trim(), all_rows)
            .await?;
    ctx.log.info(format!("Готово. Записано строк: {written}"));

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} остатков на лист «{dest_sheet}»"),
    })
}


async fn fetch_stocks(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    filter: &CabinetFilter,
) -> Result<Vec<Value>, String> {
    let mut nm: Vec<i64> = filter.nm_ids.iter().copied().collect();
    nm.sort_unstable();
    // все chrtId кабинета на каждый nm-батч.
    let mut chrt: Vec<i64> = filter.chrt_ids.iter().copied().collect();
    chrt.sort_unstable();
    ctx.log.info(format!(
        "Кабинет {cabinet}: nmId={}, chrtId={}",
        nm.len(),
        chrt.len()
    ));

    let mut out = Vec::new();
    let profile = rate::profile();
    let batches: Vec<&[i64]> = nm.chunks(NM_BATCH).collect();
    let batch_count = batches.len();

    for (bi, chunk) in batches.into_iter().enumerate() {
        let mut offset: i64 = 0;
        let mut attempts: u32 = 0;
        loop {
            let body = json!({
                "nmIds": chunk,
                "chrtIds": chrt,
                "limit": PAGE_LIMIT,
                "offset": offset,
            });
            ctx.log.info(format!(
                "Кабинет {cabinet}: запрос батч {}/{batch_count}, offset={offset}",
                bi + 1
            ));
            let v = match ctx
                .request
                .post_json(
                    Limit {
                        cabinet,
                        scope: rate::SCOPE,
                        profile: &profile,
                    },
                    STOCKS_URL,
                    Auth::Raw(token),
                    &body,
                )
                .await
            {
                Ok(v) => {
                    attempts = 0;
                    v
                }
                Err(e) => {
                    attempts += 1;
                    let retryable = is_retryable_stocks_err(&e);
                    ctx.log.warn(format!(
                        "Кабинет {cabinet}: {e} ({attempts}/{})",
                        rate::MAX_INVALID_RETRIES
                    ));
                    if !retryable || attempts >= rate::MAX_INVALID_RETRIES {
                        return Err(format!("Кабинет {cabinet}: {e}"));
                    }
                    sleep(rate::PAUSE_ON_5XX).await;
                    continue;
                }
            };
            let mut v = v;
            let items = v
                .pointer_mut("/data/items")
                .and_then(|a| a.as_array_mut())
                .map(std::mem::take)
                .unwrap_or_default();
            let n = items.len() as i64;
            ctx.log.info(format!(
                "Кабинет {cabinet}: батч {}/{batch_count}, offset={offset}, items={n}",
                bi + 1
            ));
            out.extend(items);
            offset += n;
            if n == 0 || n < PAGE_LIMIT {
                break;
            }
        }
    }
    Ok(out)
}

fn is_retryable_stocks_err(e: &str) -> bool {
    let t = e.to_ascii_lowercase();
    t.contains("timed out")
        || t.contains("timeout")
        || t.contains("http 5")
        || t.contains("http 429")
        || t.contains("connection")
        || t.contains("empty")
}

