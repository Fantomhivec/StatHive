pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_row, build_rows_header, default_fields};
use crate::scripts::ozon_util::{
    identifier_batches, json_str, ozon_post, parse_i64_list, read_identifiers_with, resolve_id_mode,
    SkuResolveCache,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v5/product/info/prices";

pub struct OzonProductPricesScript;

impl Script for OzonProductPricesScript {
    fn key(&self) -> &'static str {
        "ozon_product_prices"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_product_prices")
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
    ctx.log.info("Скрипт «OZON: Информация о цене товара»: старт");

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
            message: "Нет идентификаторов в источнике".into(),
        });
    }

    let fields = common::resolve_fields(ctx, default_fields());
    let pairs = common::load_ozon_pairs()?;
    let batches = identifier_batches(&identifiers);
    let prices_profile = crate::core::rate_profiles::ozon::product_prices::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let batches = batches.clone();
            let id_mode = id_mode.clone();
            let fields = fields.clone();
            let prices_profile = prices_profile.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log.warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                };
                let mut sku_cache = SkuResolveCache::new();
                let mut rows = Vec::new();
                let n_batches = batches.len();
                for (bi, batch) in batches.iter().enumerate() {
                    let lookup: HashMap<String, String> = batch
                        .iter()
                        .map(|id| (id.to_ascii_lowercase(), id.clone()))
                        .collect();

                    let mut use_offer = id_mode == "offer_id";
                    let mut price_ids = batch.clone();
                    let mut sku_map = HashMap::new();

                    if id_mode == "sku" {
                        let (pids, map) = sku_cache
                            .resolve(ctx, &client_id, &api_key, batch, &cabinet)
                            .await?;
                        if pids.is_empty() {
                            continue;
                        }
                        price_ids = pids;
                        sku_map = map;
                        use_offer = false;
                    }

                    let items = fetch_prices(
                        ctx,
                        &client_id,
                        &api_key,
                        &cabinet,
                        &price_ids,
                        use_offer,
                        &prices_profile,
                    )
                    .await?;
                    ctx.log.info(format!(
                        "Кабинет «{cabinet}»: батч {}/{n_batches}, цен: {}",
                        bi + 1,
                        items.len()
                    ));

                    for item in &items {
                        let mut source_id = None;
                        let mut resolved_sku = None;
                        if use_offer {
                            let oid = json_str(item.get("offer_id"));
                            source_id = lookup
                                .get(&oid.to_ascii_lowercase())
                                .cloned()
                                .or(if oid.is_empty() { None } else { Some(oid) });
                        } else if id_mode == "sku" {
                            let pid = json_str(item.get("product_id"));
                            if let Some(s) = sku_map.get(&pid) {
                                resolved_sku = Some(s.clone());
                                source_id = Some(s.clone());
                            }
                        } else {
                            let pid = json_str(item.get("product_id"));
                            source_id = lookup
                                .get(&pid.to_ascii_lowercase())
                                .cloned()
                                .or(if pid.is_empty() { None } else { Some(pid) });
                        }

                        rows.push(build_row(
                            &cabinet,
                            item,
                            &fields,
                            &id_mode,
                            source_id.as_deref(),
                            resolved_sku.as_deref(),
                        ));
                    }
                }
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut all_rows = vec![build_rows_header(&fields)];
    let mut total = 0usize;
    for (_cabinet, rows) in parts {
        total += rows.len();
        all_rows.extend(rows);
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows_with(ctx, &sheets, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк цен на лист «{sheet_name}»"),
    })
}

async fn fetch_prices(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    cabinet: &str,
    ids: &[String],
    use_offer: bool,
    profile: &crate::core::rate_limit::RateLimitProfile,
) -> Result<Vec<Value>, String> {
    let mut filter = json!({ "visibility": "ALL" });
    if use_offer {
        filter["offer_id"] = json!(ids);
    } else {
        filter["product_id"] = json!(parse_i64_list(ids));
    }

    let mut all = Vec::new();
    let mut cursor = String::new();
    let mut page = 0u32;
    let empty: Vec<Value> = Vec::new();
    loop {
        page += 1;
        let body = json!({
            "filter": filter,
            "limit": 1000,
            "cursor": cursor,
        });
        let v = ozon_post(
            ctx,
            cabinet,
            crate::core::rate_profiles::ozon::product_prices::SCOPE,
            profile,
            PATH,
            client_id,
            api_key,
            &body,
        )
        .await?;
        let items = v
            .get("items")
            .and_then(|a| a.as_array())
            .unwrap_or(&empty);
        cursor = v
            .get("cursor")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        ctx.log.info(format!(
            "Кабинет «{cabinet}»: страница {page}, items={}",
            items.len()
        ));
        all.extend(items.iter().cloned());
        if cursor.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Ok(all)
}
