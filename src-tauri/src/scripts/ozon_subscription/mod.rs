//! OZON: Добавлено в ожидаемые — POST /v1/product/info/subscription.
//! Паритет `OZ_SubscriptionScript` (только SKU).

use crate::core::rate_profiles::ozon::subscription as rate;
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_util::{
    identifier_batches, ozon_post, parse_i64_list, read_identifiers_with,
};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v1/product/info/subscription";

pub struct OzonSubscriptionScript;

impl Script for OzonSubscriptionScript {
    fn key(&self) -> &'static str {
        "ozon_subscription"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_subscription")
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
    ctx.log.info("Скрипт «OZON: Добавлено в ожидаемые»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let sheets = common::google_sheets_client(ctx).await?;
    let identifiers =
        read_identifiers_with(ctx, &sheets, &spreadsheet_id, Some("sku")).await?;
    if identifiers.is_empty() {
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет SKU в источнике".into(),
        });
    }
    ctx.log.info(format!("SKU: {}", identifiers.len()));

    let fields = common::resolve_fields(ctx, default_fields());
    let pairs = common::load_ozon_pairs()?;
    let batches = identifier_batches(&identifiers);
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let batches = batches.clone();
            let profile = profile.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log
                        .warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                };
                let mut items_all = Vec::new();
                let n_batches = batches.len();
                for (bi, batch) in batches.iter().enumerate() {
                    let skus = parse_i64_list(batch);
                    if skus.is_empty() {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: батч {}/{n_batches}, нет валидных SKU",
                            bi + 1
                        ));
                        continue;
                    }
                    let body = json!({ "skus": skus });
                    let v = ozon_post(
                        ctx,
                        &cabinet,
                        rate::SCOPE,
                        &profile,
                        PATH,
                        &client_id,
                        &api_key,
                        &body,
                    )
                    .await?;
                    let items = v
                        .get("result")
                        .and_then(|a| a.as_array())
                        .cloned()
                        .unwrap_or_default();
                    ctx.log.info(format!(
                        "Кабинет «{cabinet}»: батч {}/{n_batches}, результатов {}",
                        bi + 1,
                        items.len()
                    ));
                    items_all.extend(items);
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Ok((cabinet, items_all))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, items) in parts {
        for item in &items {
            common::ensure_header_row(&mut all_rows, &mut header_done, &fields);
            all_rows.push(build_row(item, &cabinet, &fields));
            total += 1;
        }
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows_with(ctx, &sheets, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк подписок на лист «{sheet_name}»"),
    })
}

// --- fields (merged) ---
use crate::scripts::ozon_util::json_str;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_subscription.cabinet"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_subscription.sku"), true),
        ScriptField::new("count", crate::core::i18n::t("fields.ozon_subscription.count"), true),
    ]
}

pub fn build_row(item: &Value, cabinet: &str, fields: &[ScriptField]) -> Vec<String> {
    let mut map = HashMap::new();
    map.insert("cabinet", cabinet.to_string());
    map.insert("sku", json_str(item.get("sku")));
    map.insert("count", json_str(item.get("count")));
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| map.get(f.name.as_str()).cloned().unwrap_or_default())
        .collect()
}
