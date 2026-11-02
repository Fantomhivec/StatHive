//! `claims_list` — GET returns-api `/api/v1/claims` → Sheets.

mod fields;

use crate::core::rate_profiles::claims_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const PAGE: i64 = 200;

pub struct ClaimsScript;

struct ClaimRow {
    cells: Vec<String>,
    photos: Vec<String>,
    videos: Vec<String>,
}

impl Script for ClaimsScript {
    fn key(&self) -> &'static str {
        "claims_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.claims_list")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        fields::default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Заявки на возврат»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let selected = common::resolve_fields(ctx, fields::default_fields());
    let body: Vec<&ScriptField> = selected
        .iter()
        .filter(|f| f.selected && !fields::is_media(&f.name))
        .collect();
    if body.is_empty() {
        return Err("Не выбраны поля для выгрузки".into());
    }
    let want_photos = selected.iter().any(|f| f.selected && f.name == "photos");
    let want_videos = selected.iter().any(|f| f.selected && f.name == "videos");
    let modes = archive_modes(ctx);
    let id_filter = ctx
        .params
        .get("id")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let nm_filter = ctx
        .params
        .get("nm_id")
        .or_else(|| ctx.params.get("nmId"))
        .and_then(|s| s.trim().parse::<i64>().ok());

    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            let modes = modes.clone();
            let id_filter = id_filter.clone();
            let body = body.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::new()));
                }
                let mut rows = Vec::new();
                for is_archive in modes {
                    match fetch_pages(
                        ctx,
                        &cabinet,
                        token.trim(),
                        &profile,
                        is_archive,
                        &id_filter,
                        nm_filter,
                    )
                    .await
                    {
                        Ok(list) => {
                            ctx.log.info(format!(
                                "Кабинет «{cabinet}» ({}): {} заявок",
                                if is_archive { "архив" } else { "текущие" },
                                list.len()
                            ));
                            for claim in &list {
                                let cells = body
                                    .iter()
                                    .map(|f| fields::cell(&f.name, &cabinet, claim))
                                    .collect();
                                rows.push(ClaimRow {
                                    cells,
                                    photos: if want_photos {
                                        fields::photos(claim)
                                    } else {
                                        Vec::new()
                                    },
                                    videos: if want_videos {
                                        fields::videos(claim)
                                    } else {
                                        Vec::new()
                                    },
                                });
                            }
                        }
                        Err(e) => ctx.log.warn(format!("Кабинет «{cabinet}»: {e}")),
                    }
                }
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut data = Vec::new();
    for (_cabinet, rows) in parts {
        data.extend(rows);
    }
    if data.is_empty() {
        return Ok(common::empty_data_output());
    }

    let max_photos = if want_photos {
        data.iter().map(|r| r.photos.len()).max().unwrap_or(0).min(10)
    } else {
        0
    };
    let max_videos = if want_videos {
        data.iter().map(|r| r.videos.len()).max().unwrap_or(0).min(1)
    } else {
        0
    };
    let mut header: Vec<String> = body.iter().map(|f| f.title.clone()).collect();
    for i in 1..=max_photos {
        header.push(format!("Фото {i}"));
    }
    for i in 1..=max_videos {
        header.push(format!("Видео {i}"));
    }
    let mut all_rows = vec![header];
    for row in data {
        let mut cells = row.cells;
        for i in 0..max_photos {
            cells.push(row.photos.get(i).cloned().unwrap_or_default());
        }
        for i in 0..max_videos {
            cells.push(row.videos.get(i).cloned().unwrap_or_default());
        }
        all_rows.push(cells);
    }
    let total = all_rows.len().saturating_sub(1);
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано заявок: {total} → «{sheet_name}»"),
    })
}

fn archive_modes(ctx: &ScriptContext) -> Vec<bool> {
    let raw = ctx
        .params
        .get("is_archive")
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match raw.as_str() {
        "both" | "all" => vec![false, true],
        "true" | "1" | "yes" => vec![true],
        "false" | "0" | "no" => vec![false],
        _ => {
            let archived = ctx
                .params
                .get("claims_include_archived")
                .is_some_and(|s| truthy(s));
            let current = ctx
                .params
                .get("claims_include_current")
                .map(|s| truthy(s))
                .unwrap_or(true);
            let mut modes = Vec::new();
            if current {
                modes.push(false);
            }
            if archived {
                modes.push(true);
            }
            if modes.is_empty() {
                modes.push(false);
            }
            modes
        }
    }
}

fn truthy(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

async fn fetch_pages(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    is_archive: bool,
    id_filter: &str,
    nm_filter: Option<i64>,
) -> Result<Vec<Value>, String> {
    let mut offset = 0i64;
    let mut out = Vec::new();
    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = claims_url(is_archive, offset, id_filter, nm_filter);
        let v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile,
                },
                &url,
                Auth::Bearer(token),
            )
            .await?;
        let page = v
            .get("claims")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let total = v.get("total").and_then(|t| t.as_i64()).unwrap_or(0);
        let n = page.len();
        out.extend(page);
        if n == 0 || (n as i64) < PAGE || (total > 0 && out.len() as i64 >= total) {
            break;
        }
        offset += PAGE;
    }
    Ok(out)
}

fn claims_url(is_archive: bool, offset: i64, id_filter: &str, nm_filter: Option<i64>) -> String {
    let mut q = vec![
        format!("is_archive={}", if is_archive { "true" } else { "false" }),
        format!("limit={PAGE}"),
        format!("offset={offset}"),
    ];
    if !id_filter.is_empty() {
        q.push(format!("id={}", urlencoding::encode(id_filter)));
    }
    if let Some(nm) = nm_filter {
        q.push(format!("nm_id={nm}"));
    }
    format!(
        "https://returns-api.wildberries.ru/api/v1/claims?{}",
        q.join("&")
    )
}
