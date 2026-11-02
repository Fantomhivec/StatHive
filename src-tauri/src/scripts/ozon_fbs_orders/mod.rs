pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{
    build_rows_from_posting, default_fields, export_fields_from_ctx, wants_prefix,
};
use crate::scripts::ozon_util::{
    param_bool, param_or, resolve_period_days, to_rfc3339_end, to_rfc3339_start,
};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

/// `/v3/posting/fbs/list` obsolete → `/v4/posting/fbs/list`.
/// Request: `sort_dir`, `cursor`, `limit` 1..=100; response: `postings`/`has_next`/`cursor` на корне.
const PATH: &str = "/v4/posting/fbs/list";
const PAGE_LIMIT: i64 = 100;

pub struct OzonFbsOrdersScript;

impl Script for OzonFbsOrdersScript {
    fn key(&self) -> &'static str {
        "ozon_fbs_orders"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_fbs_orders")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «OZON: Заказы FBS»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let (start, end) = match resolve_period_days(ctx, 28) {
        Ok(t) => t,
        Err(e) => {
            return Ok(ScriptOutput {
                rows_written: 0,
                message: e,
            });
        }
    };

    let raw_fields = common::resolve_fields(ctx, default_fields());
    let fields = export_fields_from_ctx(&raw_fields, ctx);
    let flags = (
        param_bool(ctx, "ozon_fbs_orders_include_analytics_data")
            || wants_prefix(&raw_fields, "analytics_"),
        param_bool(ctx, "ozon_fbs_orders_include_financial_data")
            || wants_prefix(&raw_fields, "financial_"),
        param_bool(ctx, "ozon_fbs_orders_include_legal_info")
            || wants_prefix(&raw_fields, "legal_"),
        param_bool(ctx, "ozon_fbs_orders_include_barcodes")
            || wants_prefix(&raw_fields, "barcodes_"),
    );

    let sort_dir = normalize_sort_dir(&param_or(ctx, "ozon_fbs_orders_sort_dir", "ASC"));
    let status = ctx.params.get("ozon_fbs_orders_status").cloned();
    let translit = param_bool(ctx, "ozon_fbs_orders_include_translit");

    let pairs = common::load_ozon_pairs()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let sort_dir = sort_dir.clone();
            let status = status.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log.warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                };
                let postings = fetch_postings(
                    ctx,
                    &client_id,
                    &api_key,
                    &cabinet,
                    &start,
                    &end,
                    &sort_dir,
                    status.as_deref(),
                    flags,
                    translit,
                )
                .await?;
                Ok((cabinet, postings))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = vec![fields.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, postings) in parts {
        for posting in &postings {
            for row in build_rows_from_posting(&cabinet, posting, &fields, flags) {
                all_rows.push(row);
                total += 1;
            }
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк FBS-заказов на лист «{sheet_name}»"),
    })
}

fn normalize_sort_dir(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "DESC" => "DESC".into(),
        _ => "ASC".into(),
    }
}

async fn fetch_postings(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    cabinet: &str,
    start: &chrono::NaiveDate,
    end: &chrono::NaiveDate,
    sort_dir: &str,
    status: Option<&str>,
    flags: (bool, bool, bool, bool),
    translit: bool,
) -> Result<Vec<Value>, String> {
    let (analytics, financial, legal, barcodes) = flags;

    let mut filter = json!({
        "since": to_rfc3339_start(*start),
        "to": to_rfc3339_end(*end),
    });
    if let Some(st) = status.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        filter["statuses"] = json!([st]);
    }

    let mut with = json!({});
    if analytics {
        with["analytics_data"] = json!(true);
    }
    if financial {
        with["financial_data"] = json!(true);
    }
    if legal {
        with["legal_info"] = json!(true);
    }
    if barcodes {
        with["barcodes"] = json!(true);
    }
    if translit {
        with["translit"] = json!(true);
    }

    let profile = crate::core::rate_profiles::ozon::fbs_orders::profile();
    crate::scripts::ozon_postings_run::fetch_postings_pages(
        ctx,
        cabinet,
        client_id,
        api_key,
        PATH,
        crate::core::rate_profiles::ozon::fbs_orders::SCOPE,
        &profile,
        |cursor| {
            let mut body = json!({
                "sort_dir": sort_dir,
                "filter": filter,
                "limit": PAGE_LIMIT,
                "cursor": cursor,
            });
            if with.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                body["with"] = with.clone();
            }
            body
        },
    )
    .await
}
