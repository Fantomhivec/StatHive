//! `feedbacks_archive` — GET feedbacks-api `/api/v1/feedbacks/archive`.

use super::fields;
use crate::core::rate_profiles::feedbacks_archive as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const TAKE: i64 = 5000;

pub struct FeedbacksArchiveScript;

impl Script for FeedbacksArchiveScript {
    fn key(&self) -> &'static str {
        "feedbacks_archive"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.feedbacks_archive")
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

struct Built {
    cells: Vec<String>,
    photos: Vec<String>,
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Архив отзывов»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let selected = common::resolve_fields(ctx, fields::default_fields());
    let cols: Vec<&ScriptField> = selected
        .iter()
        .filter(|f| f.selected && f.name != "photoLinks")
        .collect();
    if cols.is_empty() && !selected.iter().any(|f| f.selected && f.name == "photoLinks") {
        return Err("Не выбраны поля для выгрузки".into());
    }
    let want_photos = selected.iter().any(|f| f.selected && f.name == "photoLinks");
    let order = archive_order(ctx);
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();

    let built = if nm_filter_ready(ctx) {
        ctx.log.info("Режим: фильтр по nmId из второй таблицы");
        fetch_by_nm(ctx, &selected, &wb_tokens, &profile, &order).await?
    } else {
        let cabinets = common::resolve_cabinets(ctx);
        if cabinets.is_empty() {
            return Ok(common::empty_cabinets_output());
        }
        ctx.log.info("Режим: все архивные отзывы выбранных кабинетов");
        fetch_by_cabinets(ctx, &cabinets, &selected, &wb_tokens, &profile, &order).await?
    };

    if built.is_empty() {
        return Ok(common::empty_data_output());
    }
    let max_photos = if want_photos {
        built.iter().map(|r| r.photos.len()).max().unwrap_or(0).max(1)
    } else {
        0
    };
    let mut header: Vec<String> = cols.iter().map(|f| f.title.clone()).collect();
    for i in 1..=max_photos {
        header.push(format!("Ссылка на фото {i}"));
    }
    let mut all_rows = vec![header];
    for row in built {
        let mut cells = row.cells;
        for i in 0..max_photos {
            cells.push(row.photos.get(i).cloned().unwrap_or_default());
        }
        all_rows.push(cells);
    }
    let total = all_rows.len().saturating_sub(1);
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано архивных отзывов: {total} → «{sheet_name}»"),
    })
}

fn archive_order(ctx: &ScriptContext) -> String {
    match ctx
        .params
        .get("feedbacks_archive_order")
        .map(|s| s.trim())
    {
        Some(s) if s.eq_ignore_ascii_case("dateAsc") => "dateAsc".into(),
        _ => "dateDesc".into(),
    }
}

fn nm_filter_ready(ctx: &ScriptContext) -> bool {
    let sid = param(ctx, "second_spreadsheet_id");
    let sheet = param(ctx, "second_sheet_name");
    let cabs = param(ctx, "cabinets_source_range");
    let nms = param(ctx, "nm_id_source_range");
    !sid.is_empty() && !sheet.is_empty() && !cabs.is_empty() && !nms.is_empty()
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> &'a str {
    ctx.params.get(key).map(|s| s.trim()).unwrap_or("")
}

fn push_feedbacks(out: &mut Vec<Built>, list: &[Value], cabinet: &str, fields: &[ScriptField]) {
    for fb in list {
        let mut cells = Vec::new();
        let mut photos = Vec::new();
        let mut want = false;
        for f in fields {
            if !f.selected {
                continue;
            }
            if f.name == "photoLinks" {
                want = true;
                continue;
            }
            cells.push(fields::cell(&f.name, fb, cabinet));
        }
        if want {
            photos = fields::extract_photos(fb);
        }
        out.push(Built { cells, photos });
    }
}

async fn fetch_by_cabinets(
    ctx: &ScriptContext,
    cabinets: &[String],
    fields: &[ScriptField],
    tokens: &common::WbTokenMap,
    profile: &crate::core::rate_limit::RateLimitProfile,
    order: &str,
) -> Result<Vec<Built>, String> {
    let parts = common::map_cabinets_chunked(cabinets, common::CABINET_PARALLELISM, |cabinet| {
        let token = common::resolve_wb_token_with_map(ctx, tokens, &cabinet);
        let profile = profile.clone();
        let order = order.to_string();
        async move {
            if token.trim().is_empty() {
                ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                return Ok((cabinet, Vec::new()));
            }
            let mut skip = 0i64;
            let mut all = Vec::new();
            loop {
                ctx.check_cancelled().map_err(|e| e.to_string())?;
                match fetch_page(ctx, &cabinet, token.trim(), &profile, &order, skip, None).await {
                    Ok(page) => {
                        let n = page.len() as i64;
                        all.extend(page);
                        if n < TAKE {
                            break;
                        }
                        skip += TAKE;
                    }
                    Err(e) if is_unprocessable(&e) => {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: HTTP 422 — конец архива (skip={skip})"
                        ));
                        break;
                    }
                    Err(e) => {
                        ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                        break;
                    }
                }
            }
            ctx.log
                .info(format!("Кабинет «{cabinet}»: {} отзывов", all.len()));
            Ok((cabinet, all))
        }
    })
    .await?;
    let mut out = Vec::new();
    for (cabinet, list) in parts {
        push_feedbacks(&mut out, &list, &cabinet, fields);
    }
    Ok(out)
}

async fn fetch_by_nm(
    ctx: &ScriptContext,
    fields: &[ScriptField],
    tokens: &common::WbTokenMap,
    profile: &crate::core::rate_limit::RateLimitProfile,
    order: &str,
) -> Result<Vec<Built>, String> {
    let client = common::google_sheets_client(ctx).await?;
    let sid = param(ctx, "second_spreadsheet_id");
    let sheet = param(ctx, "second_sheet_name");
    let cols = sheet_ranges::read_first_cols(
        &client,
        sid,
        sheet,
        &[
            param(ctx, "cabinets_source_range"),
            param(ctx, "nm_id_source_range"),
        ],
    )
    .await?;
    let cabs = cols.first().cloned().unwrap_or_default();
    let nms = cols.get(1).cloned().unwrap_or_default();
    if cabs.is_empty() || nms.is_empty() {
        return Err("В диапазонах кабинетов/артикулов нет данных".into());
    }
    if cabs.len() != nms.len() {
        return Err("Количество кабинетов и артикулов в диапазонах должно совпадать".into());
    }
    let mut by_cab: HashMap<String, Vec<i64>> = HashMap::new();
    for (cab, nm_raw) in cabs.into_iter().zip(nms) {
        let cab = sheet_ranges::normalize_cabinet_name(&cab);
        if cab.is_empty() {
            continue;
        }
        let Some(nm) = sheet_ranges::parse_i64_cell(&nm_raw).filter(|n| *n > 0) else {
            continue;
        };
        by_cab.entry(cab).or_default().push(nm);
    }
    if by_cab.is_empty() {
        return Err("Нет валидных пар кабинет/артикул в диапазонах".into());
    }
    let cabinets: Vec<String> = by_cab.keys().cloned().collect();
    let parts = common::map_cabinets_chunked(&cabinets, common::CABINET_PARALLELISM, |cabinet| {
        let nms = by_cab.get(&cabinet).cloned().unwrap_or_default();
        let token = common::resolve_wb_token_with_map(ctx, tokens, &cabinet);
        let profile = profile.clone();
        let order = order.to_string();
        async move {
            if token.trim().is_empty() {
                ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                return Ok((cabinet, Vec::new()));
            }
            let mut all = Vec::new();
            for nm in nms {
                ctx.check_cancelled().map_err(|e| e.to_string())?;
                match fetch_page(ctx, &cabinet, token.trim(), &profile, &order, 0, Some(nm)).await
                {
                    Ok(page) => all.extend(page),
                    Err(e) if is_unprocessable(&e) => {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}» nmId={nm}: HTTP 422 — пустой ответ"
                        ));
                    }
                    Err(e) => ctx.log.warn(format!("Кабинет «{cabinet}» nmId={nm}: {e}")),
                }
            }
            Ok((cabinet, all))
        }
    })
    .await?;
    let mut out = Vec::new();
    for (cabinet, list) in parts {
        push_feedbacks(&mut out, &list, &cabinet, fields);
    }
    Ok(out)
}

async fn fetch_page(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    order: &str,
    skip: i64,
    nm: Option<i64>,
) -> Result<Vec<Value>, String> {
    let mut url = format!(
        "https://feedbacks-api.wildberries.ru/api/v1/feedbacks/archive?take={TAKE}&skip={skip}&order={}",
        urlencoding::encode(order)
    );
    if let Some(nm) = nm {
        url.push_str(&format!("&nmId={nm}"));
    }
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
    Ok(v.get("data")
        .and_then(|d| d.get("feedbacks"))
        .and_then(|f| f.as_array())
        .cloned()
        .unwrap_or_default())
}

fn is_unprocessable(err: &str) -> bool {
    err.contains("HTTP 422")
}
