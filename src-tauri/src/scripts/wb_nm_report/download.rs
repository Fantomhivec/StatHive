//! `nm_report_download_file`.

use super::api;
use super::api::download_fields;
use crate::core::rate_profiles::nm_report::file as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::sheet_ranges::{self, normalize_cabinet_name};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct NmReportDownloadFileScript;

impl Script for NmReportDownloadFileScript {
    fn key(&self) -> &'static str {
        "nm_report_download_file"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.nm_report_download_file")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        download_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> Option<&'a str> {
    ctx.params
        .get(key)
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Nm-report file: старт");
    let (dest_id, dest_sheet) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let src_id = param(ctx, "second_spreadsheet_id")
        .or_else(|| param(ctx, "source_spreadsheet_id"))
        .unwrap_or(dest_id.as_str());
    let src_sheet = param(ctx, "second_sheet_name")
        .or_else(|| param(ctx, "source_sheet_name"))
        .unwrap_or(dest_sheet.as_str());
    let cab_r = param(ctx, "cabinets_source_range").unwrap_or("");
    let id_r = param(ctx, "paid_storage_download_task_id_range").unwrap_or("");
    if cab_r.is_empty() || id_r.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Укажите диапазоны кабинетов и downloadId (поле taskId)".into(),
        });
    }

    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(&client, src_id, src_sheet, &[cab_r, id_r]).await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let ids = cols.get(1).cloned().unwrap_or_default();
    if cabs.len() != ids.len() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: format!(
                "Длины диапазонов кабинетов ({}) и downloadId ({}) должны совпадать",
                cabs.len(),
                ids.len()
            ),
        });
    }

    let mut by_cab: HashMap<String, Vec<String>> = HashMap::new();
    for (c, id) in cabs.into_iter().zip(ids) {
        let cab = normalize_cabinet_name(&c);
        let id = id.trim().to_string();
        if cab.is_empty() || id.is_empty() {
            continue;
        }
        by_cab.entry(cab).or_default().push(id);
    }
    if by_cab.is_empty() {
        return Ok(common::empty_data_output());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let cab_order: Vec<String> = by_cab.keys().cloned().collect();
    let parts = common::map_cabinets_chunked(
        &cab_order,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let ids = by_cab.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                }
                let mut all = Vec::new();
                let mut header: Option<Vec<String>> = None;
                for id in ids {
                    match api::download_file_rows(
                        &ctx.request,
                        &cabinet,
                        token.trim(),
                        rate::SCOPE,
                        &profile,
                        &id,
                    )
                    .await
                    {
                        Ok(mut rows) => {
                            if rows.is_empty() {
                                continue;
                            }
                            if header.is_none() {
                                let mut h = rows.remove(0);
                                h.insert(0, "Кабинет".into());
                                header = Some(h);
                            } else {
                                rows.remove(0);
                            }
                            for mut r in rows {
                                r.insert(0, cabinet.clone());
                                all.push(r);
                            }
                        }
                        Err(e) => ctx.log.warn(format!("downloadId={id}: {e}")),
                    }
                }
                let mut out = Vec::new();
                if let Some(h) = header {
                    out.push(h);
                }
                out.extend(all);
                Ok((cabinet, out))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (_cab, rows) in parts {
        if rows.is_empty() {
            continue;
        }
        let data_n = rows.len().saturating_sub(1);
        total += data_n;
        common::append_cabinet_rows(&mut all_rows, &mut header_done, rows);
    }
    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &dest_id, &dest_sheet, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк nm-report на «{dest_sheet}»"),
    })
}
