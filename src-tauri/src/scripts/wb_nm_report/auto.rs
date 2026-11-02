//! `nm_report_downloads_auto` — create DETAIL_HISTORY → poll → download.

use super::api;
use super::api::download_fields;
use crate::core::rate_limit::RateLimitProfile;
use crate::core::rate_profiles::nm_report::{auto as rate_detail, stock_history_auto as rate_stock};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use std::future::Future;
use std::pin::Pin;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

struct AutoRates {
    scope_create: &'static str,
    scope_list: &'static str,
    scope_file: &'static str,
    create: RateLimitProfile,
    list: RateLimitProfile,
    file: RateLimitProfile,
}

fn rates_for(stock_history: bool) -> AutoRates {
    if stock_history {
        AutoRates {
            scope_create: rate_stock::SCOPE_CREATE,
            scope_list: rate_stock::SCOPE_LIST,
            scope_file: rate_stock::SCOPE_FILE,
            create: rate_stock::profile_create(),
            list: rate_stock::profile_list(),
            file: rate_stock::profile_file(),
        }
    } else {
        AutoRates {
            scope_create: rate_detail::SCOPE_CREATE,
            scope_list: rate_detail::SCOPE_LIST,
            scope_file: rate_detail::SCOPE_FILE,
            create: rate_detail::profile_create(),
            list: rate_detail::profile_list(),
            file: rate_detail::profile_file(),
        }
    }
}

pub struct NmReportDownloadsAutoScript;

impl Script for NmReportDownloadsAutoScript {
    fn key(&self) -> &'static str {
        "nm_report_downloads_auto"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.nm_report_downloads_auto")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        download_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx, false).await })
    }
}

struct TaskInfo {
    cabinet: String,
    download_id: String,
}

pub(crate) async fn run(ctx: &ScriptContext, stock_history: bool) -> ScriptResult {
    let label = if stock_history {
        "Nm-report stock-history auto"
    } else {
        "Nm-report auto"
    };
    ctx.log.info(format!("{label}: старт"));
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (start, end) = crate::scripts::dates::resolve_period(ctx, 14)?;
    if stock_history {
        if (end - start).num_days() > 92 {
            return Err("История остатков: максимум ~3 месяца.".into());
        }
    } else if (end - start).num_days() > 365 {
        return Err("Максимум 365 дней для DETAIL_HISTORY_REPORT.".into());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let wb_tokens = common::load_wb_tokens()?;
    let rates = rates_for(stock_history);
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let create_p = rates.create.clone();
            let scope_create = rates.scope_create;
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, None::<String>));
                }
                let id = Uuid::new_v4().to_string();
                let res = if stock_history {
                    api::create_stock_history_daily(
                        &ctx.request,
                        &cabinet,
                        token.trim(),
                        scope_create,
                        &create_p,
                        &id,
                        start,
                        end,
                    )
                    .await
                } else {
                    api::create_detail_history(
                        &ctx.request,
                        &cabinet,
                        token.trim(),
                        scope_create,
                        &create_p,
                        &id,
                        start,
                        end,
                    )
                    .await
                };
                match res {
                    Ok(()) => {
                        ctx.log
                            .info(format!("Кабинет «{cabinet}»: downloadId={id}"));
                        Ok((cabinet, Some(id)))
                    }
                    Err(e) => {
                        ctx.log.warn(e);
                        Ok((cabinet, None))
                    }
                }
            }
        },
    )
    .await?;

    let mut tasks: Vec<TaskInfo> = Vec::new();
    for (cabinet, id) in parts {
        if let Some(download_id) = id {
            tasks.push(TaskInfo {
                cabinet,
                download_id,
            });
        }
    }
    if tasks.is_empty() {
        return Ok(common::empty_data_output());
    }

    wait_ready(ctx, &wb_tokens, &tasks, &rates).await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for t in &tasks {
        let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &t.cabinet);
        if token.trim().is_empty() {
            continue;
        }
        match api::download_file_rows(
            &ctx.request,
            &t.cabinet,
            token.trim(),
            rates.scope_file,
            &rates.file,
            &t.download_id,
        )
        .await
        {
            Ok(mut rows) => {
                if rows.is_empty() {
                    continue;
                }
                let mut header = rows.remove(0);
                header.insert(0, "Кабинет".into());
                let mut block = vec![header];
                for mut r in rows {
                    r.insert(0, t.cabinet.clone());
                    block.push(r);
                }
                total += block.len().saturating_sub(1);
                common::append_cabinet_rows(&mut all_rows, &mut header_done, block);
            }
            Err(e) => ctx.log.warn(format!("{}: {e}", t.download_id)),
        }
    }
    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Авто nm-report: {total} строк на «{sheet_name}»"),
    })
}

async fn wait_ready(
    ctx: &ScriptContext,
    wb_tokens: &std::collections::HashMap<String, String>,
    tasks: &[TaskInfo],
    rates: &AutoRates,
) -> Result<(), String> {
    for _ in 0..90 {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let mut pending = 0usize;
        for t in tasks {
            let token = common::resolve_wb_token_with_map(ctx, wb_tokens, &t.cabinet);
            if token.trim().is_empty() {
                continue;
            }
            let items = api::list_downloads(
                &ctx.request,
                &t.cabinet,
                token.trim(),
                rates.scope_list,
                &rates.list,
                &[t.download_id.clone()],
            )
            .await
            .unwrap_or_default();
            let status = items
                .first()
                .map(api::item_status)
                .unwrap_or_else(|| "unknown".into());
            if api::is_failed(&status) {
                return Err(format!(
                    "Кабинет «{}»: отчёт {} failed ({status})",
                    t.cabinet, t.download_id
                ));
            }
            if !api::is_success(&status) {
                pending += 1;
            }
        }
        if pending == 0 {
            return Ok(());
        }
        ctx.log
            .info(format!("Ожидание готовности nm-report: осталось {pending}"));
        sleep(Duration::from_secs(20)).await;
    }
    Err("Таймаут ожидания готовности nm-report".into())
}

pub struct NmReportStockHistoryAutoScript;

impl Script for NmReportStockHistoryAutoScript {
    fn key(&self) -> &'static str {
        "nm_report_stock_history_auto"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.nm_report_stock_history_auto")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        download_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx, true).await })
    }
}
