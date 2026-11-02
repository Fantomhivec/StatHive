//! Общий runner POST `/v3/product/info/list` по ID-батчам (product_info / commissions / stocks).

use crate::core::rate_limit::RateLimitProfile;
use crate::core::script::{ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common::{self, OzonPairMap};
use crate::scripts::ozon_util::{
    identifier_batches, ozon_post, product_info_list_body, read_identifiers_with, resolve_id_mode,
};
use serde_json::Value;

const PATH: &str = "/v3/product/info/list";

pub struct ListRunParams<'a> {
    pub script_label: &'a str,
    pub scope: &'a str,
    pub profile: RateLimitProfile,
    pub empty_ids_message: &'a str,
    /// Фрагмент: «информации о товарах» / «комиссий» / «остатков».
    pub rows_label: &'a str,
}

/// Одна строка на item (product_info / commissions).
pub async fn run_one_row_per_item<F>(
    ctx: &ScriptContext,
    params: ListRunParams<'_>,
    fields: Vec<ScriptField>,
    mut build_row: F,
) -> ScriptResult
where
    F: FnMut(&Value, &str, &[ScriptField], &str, Option<&str>) -> Vec<String> + Send,
{
    run_inner(
        ctx,
        params,
        fields,
        |all, header_done, fields, cab, mode, item, src| {
            let row = build_row(item, cab, fields, mode, src);
            common::ensure_header_row(all, header_done, fields);
            all.push(row);
            1
        },
    )
    .await
}

/// 1→N строк на item (stocks через append).
pub async fn run_multi_rows_per_item<F>(
    ctx: &ScriptContext,
    params: ListRunParams<'_>,
    fields: Vec<ScriptField>,
    mut append_item: F,
) -> ScriptResult
where
    F: FnMut(&mut Vec<Vec<String>>, &mut bool, &str, &Value, &[ScriptField], &str, Option<&str>)
        + Send,
{
    run_inner(
        ctx,
        params,
        fields,
        |all, header_done, fields, cab, mode, item, src| {
            let before = all.len();
            append_item(all, header_done, cab, item, fields, mode, src);
            all.len().saturating_sub(before)
        },
    )
    .await
}

async fn run_inner<F>(
    ctx: &ScriptContext,
    params: ListRunParams<'_>,
    fields: Vec<ScriptField>,
    mut on_item: F,
) -> ScriptResult
where
    F: FnMut(
            &mut Vec<Vec<String>>,
            &mut bool,
            &[ScriptField],
            &str,
            &str,
            &Value,
            Option<&str>,
        ) -> usize
        + Send,
{
    ctx.log
        .info(format!("Скрипт «{}»: старт", params.script_label));

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let id_mode = resolve_id_mode(ctx);
    if !matches!(id_mode.as_str(), "offer_id" | "product_id" | "sku") {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "id_mode: offer_id, product_id или sku".into(),
        });
    }

    let sheets = common::google_sheets_client(ctx).await?;
    let identifiers =
        read_identifiers_with(ctx, &sheets, &spreadsheet_id, Some(&id_mode)).await?;
    if identifiers.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: params.empty_ids_message.into(),
        });
    }
    ctx.log.info(format!(
        "Идентификаторов: {}, режим {id_mode}",
        identifiers.len()
    ));

    let pairs: OzonPairMap = common::load_ozon_pairs()?;
    let batches = identifier_batches(&identifiers);
    let profile = params.profile;
    let scope = params.scope;
    let n_batches = batches.len();

    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let batches = batches.clone();
            let id_mode = id_mode.clone();
            let profile = profile.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log
                        .warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<(Vec<String>, Vec<Value>)>::new()));
                };
                let mut out = Vec::new();
                for (bi, batch) in batches.iter().enumerate() {
                    let body = product_info_list_body(&id_mode, batch);
                    let v = ozon_post(
                        ctx,
                        &cabinet,
                        scope,
                        &profile,
                        PATH,
                        &client_id,
                        &api_key,
                        &body,
                    )
                    .await?;
                    let items = v
                        .get("items")
                        .and_then(|a| a.as_array())
                        .cloned()
                        .unwrap_or_default();
                    ctx.log.info(format!(
                        "Кабинет «{cabinet}»: батч {}/{n_batches}, товаров {}",
                        bi + 1,
                        items.len()
                    ));
                    out.push((batch.clone(), items));
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Ok((cabinet, out))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, cab_batches) in parts {
        for (batch, items) in cab_batches {
            for (idx, item) in items.iter().enumerate() {
                let src = batch.get(idx).map(|s| s.as_str());
                total += on_item(
                    &mut all_rows,
                    &mut header_done,
                    &fields,
                    &cabinet,
                    &id_mode,
                    item,
                    src,
                );
            }
        }
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows_with(ctx, &sheets, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!(
            "Записано {total} строк {} на лист «{sheet_name}»",
            params.rows_label
        ),
    })
}
