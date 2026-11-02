use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_util::{
    identifier_batches, ozon_post, param_or, parse_i64_list, read_identifiers_with,
    resolve_id_mode, SkuResolveCache,
};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v4/product/info/stocks";

pub struct OzonFbsStocksScript;

impl Script for OzonFbsStocksScript {
    fn key(&self) -> &'static str {
        "ozon_fbs_stocks"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_fbs_stocks")
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
    ctx.log.info("Скрипт «OZON: Остатки FBS»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let mut id_mode = resolve_id_mode(ctx);
    if !matches!(id_mode.as_str(), "offer_id" | "product_id" | "sku") {
        id_mode = "sku".into();
    }
    let use_sku = id_mode == "sku";
    let mut use_offer = id_mode == "offer_id";
    let mut use_product = id_mode == "product_id";
    if crate::scripts::ozon_util::param_bool(ctx, "ozon_fbs_stocks_use_offer_id") {
        use_offer = true;
        use_product = false;
    } else if crate::scripts::ozon_util::param_bool(ctx, "ozon_fbs_stocks_use_product_id") {
        use_product = true;
        use_offer = false;
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

    let visibility = param_or(ctx, "ozon_fbs_stocks_visibility", "ALL");
    let fields = common::resolve_fields(ctx, default_fields());
    let pairs = common::load_ozon_pairs()?;
    let batches = identifier_batches(&identifiers);
    let profile = crate::core::rate_profiles::ozon::fbs_stocks::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let batches = batches.clone();
            let fields = fields.clone();
            let visibility = visibility.clone();
            let profile = profile.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log.warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                };
                let mut sku_cache = SkuResolveCache::new();
                let mut rows_out = Vec::new();
                let n_batches = batches.len();
                for (bi, batch) in batches.iter().enumerate() {
                    let mut product_batch = batch.clone();
                    let mut cab_use_offer = use_offer;
                    let mut cab_use_product = use_product;
                    if use_sku {
                        let (pids, _) = sku_cache
                            .resolve(ctx, &client_id, &api_key, batch, &cabinet)
                            .await?;
                        if pids.is_empty() {
                            continue;
                        }
                        product_batch = pids;
                        cab_use_product = true;
                        cab_use_offer = false;
                    }

                    let mut cursor = String::new();
                    let mut page = 0u32;
                    loop {
                        page += 1;
                        let filter = build_filter(
                            cab_use_offer,
                            cab_use_product,
                            &product_batch,
                            batch,
                            &visibility,
                        );
                        let mut body = json!({ "filter": filter, "limit": 1000 });
                        if !cursor.is_empty() {
                            body["cursor"] = json!(cursor);
                        }

                        let v = ozon_post(
                            ctx,
                            &cabinet,
                            crate::core::rate_profiles::ozon::fbs_stocks::SCOPE,
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
                        cursor = v
                            .get("cursor")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();

                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: батч {}/{n_batches}, стр. {page}, items={}",
                            bi + 1,
                            items.len()
                        ));

                        for item in &items {
                            let src = item
                                .get("offer_id")
                                .or_else(|| item.get("product_id"))
                                .map(json_str_val);
                            let rows = build_rows(
                                &cabinet,
                                item,
                                &fields,
                                cab_use_offer,
                                cab_use_product,
                                src.as_deref(),
                            );
                            rows_out.extend(rows);
                        }

                        if cursor.is_empty() {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                }
                Ok((cabinet, rows_out))
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
        common::ensure_header_row(&mut all_rows, &mut header_done, &fields);
        total += rows.len();
        all_rows.extend(rows);
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows_with(ctx, &sheets, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк FBS-остатков на лист «{sheet_name}»"),
    })
}

fn json_str_val(v: &Value) -> String {
    crate::scripts::ozon_util::json_str(Some(v))
}

fn build_filter(
    use_offer: bool,
    use_product: bool,
    product_batch: &[String],
    offer_batch: &[String],
    visibility: &str,
) -> Value {
    let mut filter = json!({});
    if use_offer {
        filter["offer_id"] = json!(offer_batch);
    }
    if use_product {
        filter["product_id"] = json!(parse_i64_list(product_batch));
    }
    if !visibility.is_empty() && visibility != "ALL" {
        filter["visibility"] = json!(visibility);
    }
    filter
}

// --- fields (merged) ---
use crate::scripts::ozon_util::json_str;
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_fbs_stocks.cabinet"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_fbs_stocks.product_id"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_fbs_stocks.offer_id"), true),
        ScriptField::new("present", crate::core::i18n::t("fields.ozon_fbs_stocks.present"), true),
        ScriptField::new("reserved", crate::core::i18n::t("fields.ozon_fbs_stocks.reserved"), true),
        ScriptField::new("shipment_type", crate::core::i18n::t("fields.ozon_fbs_stocks.shipment_type"), true),
        ScriptField::new("sku", crate::core::i18n::t("fields.ozon_fbs_stocks.sku"), true),
        ScriptField::new("type", crate::core::i18n::t("fields.ozon_fbs_stocks.type"), true),
    ]
}

fn cell(map: &HashMap<&str, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_rows(
    cabinet: &str,
    item: &Value,
    fields: &[ScriptField],
    use_offer: bool,
    use_product: bool,
    source_id: Option<&str>,
) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut offer_id = json_str(item.get("offer_id"));
    let mut product_id = json_str(item.get("product_id"));
    if let Some(src) = source_id {
        if use_offer && offer_id.is_empty() {
            offer_id = src.to_string();
        } else if use_product && product_id.is_empty() {
            product_id = src.to_string();
        }
    }

    let stocks = item
        .get("stocks")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    if stocks.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    for stock in &stocks {
        let mut map = HashMap::new();
        map.insert("cabinet", cabinet.to_string());
        map.insert("offer_id", offer_id.clone());
        map.insert("product_id", product_id.clone());
        map.insert("present", json_str(stock.get("present")));
        map.insert("reserved", json_str(stock.get("reserved")));
        map.insert("shipment_type", json_str(stock.get("shipment_type")));
        map.insert("sku", json_str(stock.get("sku")));
        map.insert("type", json_str(stock.get("type")));
        rows.push(selected.iter().map(|f| cell(&map, &f.name)).collect());
    }
    rows
}
