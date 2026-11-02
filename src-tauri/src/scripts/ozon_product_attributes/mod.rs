pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{build_row, default_fields};
use crate::scripts::ozon_util::{
    identifier_batches, ozon_post, param_or, read_identifiers, resolve_id_mode,
};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v4/product/info/attributes";

pub struct OzonProductAttributesScript;

impl Script for OzonProductAttributesScript {
    fn key(&self) -> &'static str {
        "ozon_product_attributes"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_product_attributes")
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
    ctx.log.info("Скрипт «OZON: Характеристики товара»: старт");

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

    let identifiers = read_identifiers(ctx, &spreadsheet_id).await?;
    if identifiers.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет идентификаторов в источнике".into(),
        });
    }

    let visibility = param_or(ctx, "ozon_product_attributes_visibility", "ALL");
    let sort_by = param_or(ctx, "ozon_product_attributes_sort_by", "sku");
    let sort_dir = param_or(ctx, "ozon_product_attributes_sort_dir", "asc");

    let fields = common::resolve_fields(ctx, default_fields());
    let pairs = common::load_ozon_pairs()?;
    let batches = identifier_batches(&identifiers);
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let batches = batches.clone();
            let id_mode = id_mode.clone();
            let fields = fields.clone();
            let visibility = visibility.clone();
            let sort_by = sort_by.clone();
            let sort_dir = sort_dir.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log.warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                };
                let mut rows = Vec::new();
                let n_batches = batches.len();
                for (bi, batch) in batches.iter().enumerate() {
                    let source_ids: Vec<String> = match id_mode.as_str() {
                        "product_id" | "sku" => batch
                            .iter()
                            .filter(|s| s.trim().parse::<i64>().is_ok())
                            .cloned()
                            .collect(),
                        _ => batch.clone(),
                    };
                    if source_ids.is_empty() {
                        continue;
                    }

                    let mut last_id = String::new();
                    loop {
                        let filter = filter_for(&id_mode, &source_ids);
                        let mut body = json!({
                            "filter": filter,
                            "visibility": visibility,
                            "sort_by": sort_by,
                            "sort_dir": sort_dir,
                            "limit": 1000,
                        });
                        if !last_id.is_empty() {
                            body["last_id"] = json!(last_id);
                        }

                        let profile =
                            crate::core::rate_profiles::ozon::product_attributes::profile();
                        let v = ozon_post(
                            ctx,
                            &cabinet,
                            crate::core::rate_profiles::ozon::product_attributes::SCOPE,
                            &profile,
                            PATH,
                            &client_id,
                            &api_key,
                            &body,
                        )
                        .await?;
                        let result = v
                            .get("result")
                            .and_then(|a| a.as_array())
                            .cloned()
                            .unwrap_or_default();
                        last_id = v
                            .get("last_id")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();

                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: батч {}/{n_batches}, result={}, last_id={last_id}",
                            bi + 1,
                            result.len()
                        ));

                        for (idx, item) in result.iter().enumerate() {
                            let src = source_ids.get(idx).map(|s| s.as_str());
                            rows.push(build_row(&cabinet, item, &fields, &id_mode, src));
                        }

                        if last_id.is_empty() {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                }
                Ok((cabinet, rows))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (_cabinet, rows) in parts {
        if rows.is_empty() {
            continue;
        }
        if !header_done {
            let sel: Vec<_> = fields.iter().filter(|f| f.selected).collect();
            all_rows.push(sel.iter().map(|f| f.title.clone()).collect());
            header_done = true;
        }
        total += rows.len();
        all_rows.extend(rows);
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк характеристик на лист «{sheet_name}»"),
    })
}

fn filter_for(id_mode: &str, ids: &[String]) -> Value {
    crate::scripts::ozon_util::id_filter_json(id_mode, ids)
}
