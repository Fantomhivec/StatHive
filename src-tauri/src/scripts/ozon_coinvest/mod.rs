//! OZON: Получение Соинвест — GET /v1/actions + POST /v2/actions/candidates.
//! Паритет `OZ_CoinvestScript`.

use crate::core::rate_profiles::ozon::{coinvest_actions, coinvest_candidates};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_util::{ozon_get, ozon_post};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;

const ACTION_TITLE_NEEDLE: &str = "эластич";
const CANDIDATES_LIMIT: i64 = 100;

#[derive(Clone, Default)]
struct SchemaPrices {
    black: String,
    green: String,
    currency: String,
}

#[derive(Clone)]
struct CoinvestProduct {
    cabinet: String,
    product_id: String,
    /// schema name → prices (case-insensitive keys via lower map)
    schemas: HashMap<String, SchemaPrices>,
}

/// Ozon: соинвест (`ozon_coinvest`) → Google Sheets.
pub struct OzonCoinvestScript;

impl Script for OzonCoinvestScript {
    fn key(&self) -> &'static str {
        "ozon_coinvest"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_coinvest")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        Vec::new()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run(ctx).await })
    }
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «OZON: Получение Соинвест»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log
        .info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let pairs = common::load_ozon_pairs()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log
                        .warn(format!("OZON ключи не заданы для кабинета «{cabinet}»"));
                    return Ok((
                        cabinet,
                        Err::<Vec<CoinvestProduct>, String>("OZON ключи не заданы".into()),
                    ));
                };
                match process_cabinet(ctx, &cabinet, &client_id, &api_key).await {
                    Ok(list) => {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: получено товаров {}",
                            list.len()
                        ));
                        Ok((cabinet, Ok(list)))
                    }
                    Err(e) => {
                        ctx.log
                            .warn(format!("Кабинет «{cabinet}»: пропуск — {e}"));
                        Ok((cabinet, Err(e)))
                    }
                }
            }
        },
    )
    .await?;

    let mut products: Vec<CoinvestProduct> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut any_ok = false;
    for (cabinet, result) in parts {
        match result {
            Ok(list) => {
                any_ok = true;
                products.extend(list);
            }
            Err(e) => failures.push((cabinet, e)),
        }
    }

    products.retain(has_any_price);

    if products.is_empty() {
        if !any_ok && !failures.is_empty() {
            let msg = failures
                .iter()
                .map(|(c, e)| format!("{c}: {e}"))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("Ошибка соинвеста: {msg}").into());
        }
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Нет товаров с ценами для записи".into(),
        });
    }

    let schema_order = build_schema_order(&products);
    let headers = build_headers(&schema_order);
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(products.len() + 1);
    rows.push(headers);
    for p in &products {
        rows.push(build_data_row(p, &schema_order));
    }

    let n = products.len();
    ctx.log.info(format!(
        "Запись: {n} товаров, схем: {} ({})",
        schema_order.len(),
        schema_order.join(", ")
    ));
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, rows).await?;

    Ok(ScriptOutput {
        rows_written: n,
        message: format!("Записано {n} строк соинвеста на лист «{sheet_name}»"),
    })
}

async fn process_cabinet(
    ctx: &ScriptContext,
    cabinet: &str,
    client_id: &str,
    api_key: &str,
) -> Result<Vec<CoinvestProduct>, String> {
    ctx.log
        .info(format!("Кабинет «{cabinet}»: шаг 1 — GET /v1/actions"));
    let action_id = find_elastic_action_id(ctx, cabinet, client_id, api_key).await?;
    ctx.log
        .info(format!("Кабинет «{cabinet}»: action_id={action_id}"));
    ctx.log.info(format!(
        "Кабинет «{cabinet}»: шаг 2 — POST /v2/actions/candidates"
    ));
    fetch_candidates(ctx, cabinet, client_id, api_key, action_id).await
}

async fn find_elastic_action_id(
    ctx: &ScriptContext,
    cabinet: &str,
    client_id: &str,
    api_key: &str,
) -> Result<i64, String> {
    let profile = coinvest_actions::profile();
    let response = ozon_get(
        ctx,
        cabinet,
        coinvest_actions::SCOPE,
        &profile,
        "/v1/actions",
        client_id,
        api_key,
    )
    .await?;

    let actions = response
        .get("result")
        .and_then(|v| v.as_array())
        .or_else(|| response.get("actions").and_then(|v| v.as_array()))
        .cloned()
        .unwrap_or_default();

    let matches: Vec<&Value> = actions
        .iter()
        .filter(|a| {
            a.get("title")
                .and_then(|t| t.as_str())
                .map(|t| t.to_lowercase().contains(ACTION_TITLE_NEEDLE))
                .unwrap_or(false)
        })
        .collect();

    if matches.is_empty() {
        return Err(format!(
            "Акция с «{ACTION_TITLE_NEEDLE}» в названии не найдена"
        ));
    }

    let chosen = matches[0];
    let title = chosen
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let id = chosen
        .get("id")
        .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|n| n as i64)))
        .filter(|&n| n > 0)
        .ok_or_else(|| format!("У акции «{title}» нет корректного id"))?;

    if matches.len() > 1 {
        ctx.log.info(format!(
            "Кабинет «{cabinet}»: найдено акций с «{ACTION_TITLE_NEEDLE}»: {}, выбрана «{title}» (id={id})",
            matches.len()
        ));
    } else {
        ctx.log.info(format!(
            "Кабинет «{cabinet}»: акция «{title}» (id={id})"
        ));
    }
    Ok(id)
}

async fn fetch_candidates(
    ctx: &ScriptContext,
    cabinet: &str,
    client_id: &str,
    api_key: &str,
    action_id: i64,
) -> Result<Vec<CoinvestProduct>, String> {
    let profile = coinvest_candidates::profile();
    let mut products = Vec::new();
    let mut last_id = String::new();
    let mut page = 0u32;

    loop {
        page += 1;
        let body = json!({
            "action_id": action_id,
            "last_id": last_id,
            "limit": CANDIDATES_LIMIT
        });
        let response = ozon_post(
            ctx,
            cabinet,
            coinvest_candidates::SCOPE,
            &profile,
            "/v2/actions/candidates",
            client_id,
            api_key,
            &body,
        )
        .await?;

        let root = response.get("result").unwrap_or(&response);
        let items = root
            .get("products")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let next_last_id = root
            .get("last_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        ctx.log.info(format!(
            "Кабинет «{cabinet}»: страница {page}, товаров {}",
            items.len()
        ));

        for item in &items {
            if let Some(p) = parse_candidate(cabinet, item) {
                products.push(p);
            }
        }

        if items.is_empty() || next_last_id.trim().is_empty() || next_last_id == last_id {
            break;
        }
        last_id = next_last_id;
    }

    Ok(products)
}

fn parse_candidate(cabinet: &str, item: &Value) -> Option<CoinvestProduct> {
    let product_id = item
        .get("id")
        .map(|v| match v {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .unwrap_or_default();
    let product_id = product_id.trim().to_string();
    if product_id.is_empty() {
        return None;
    }

    let mut product = CoinvestProduct {
        cabinet: cabinet.to_string(),
        product_id,
        schemas: HashMap::new(),
    };

    let Some(prices_by_schema) = item
        .pointer("/website_prices/prices_by_schema")
        .and_then(|v| v.as_object())
    else {
        return Some(product);
    };

    for (schema_name, schema_val) in prices_by_schema {
        let schema_name = schema_name.trim();
        if schema_name.is_empty() {
            continue;
        }
        let Some(obj) = schema_val.as_object() else {
            continue;
        };
        let black = obj.get("black_price");
        let green = obj.get("green_price");
        let black_amount = black
            .and_then(|b| b.get("amount"))
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();
        let green_amount = green
            .and_then(|g| g.get("amount"))
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();
        let currency = black
            .and_then(|b| b.get("currency"))
            .and_then(|c| c.as_str())
            .or_else(|| {
                green
                    .and_then(|g| g.get("currency"))
                    .and_then(|c| c.as_str())
            })
            .unwrap_or("")
            .to_string();

        product.schemas.insert(
            schema_name.to_string(),
            SchemaPrices {
                black: black_amount,
                green: green_amount,
                currency,
            },
        );
    }

    Some(product)
}

fn has_any_price(p: &CoinvestProduct) -> bool {
    p.schemas.values().any(|s| !s.black.trim().is_empty() || !s.green.trim().is_empty())
}

fn build_schema_order(products: &[CoinvestProduct]) -> Vec<String> {
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    for p in products {
        for key in p.schemas.keys() {
            let lower = key.to_ascii_lowercase();
            names.entry(lower).or_insert_with(|| key.clone());
        }
    }

    let mut ordered = Vec::new();
    if let Some(fbs) = names.remove("fbs") {
        ordered.push(fbs);
    }
    ordered.extend(names.into_values());
    ordered
}

fn build_headers(schema_order: &[String]) -> Vec<String> {
    let mut headers = vec!["Кабинет".into(), "ID товара".into()];
    for schema in schema_order {
        headers.push(format!("{schema} Чёрная цена"));
        headers.push(format!("{schema} Зелёная цена"));
        headers.push(format!("{schema} валюта"));
    }
    headers
}

fn build_data_row(product: &CoinvestProduct, schema_order: &[String]) -> Vec<String> {
    let mut row = vec![product.cabinet.clone(), product.product_id.clone()];
    for schema in schema_order {
        if let Some(prices) = product
            .schemas
            .get(schema)
            .or_else(|| {
                product
                    .schemas
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(schema))
                    .map(|(_, v)| v)
            })
        {
            row.push(prices.black.clone());
            row.push(prices.green.clone());
            row.push(prices.currency.clone());
        } else {
            row.push(String::new());
            row.push(String::new());
            row.push(String::new());
        }
    }
    row
}
