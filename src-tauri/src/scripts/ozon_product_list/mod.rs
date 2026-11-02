use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v3/product/list";
const PAGE_LIMIT: i64 = 1000;

/// Ozon: список товаров (`ozon_product_list`) → Google Sheets.
pub struct OzonProductListScript;

impl Script for OzonProductListScript {
    fn key(&self) -> &'static str {
        "ozon_product_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_product_list")
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
    ctx.log.info("Скрипт «OZON: Список товаров»: старт");

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

    let fields = common::resolve_fields(ctx, default_fields());
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
                    return Ok((cabinet, Vec::<Value>::new()));
                };
                let items = fetch_all(ctx, &client_id, &api_key, &cabinet).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} товаров (items)", items.len()));
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, items) in parts {
        if items.is_empty() {
            continue;
        }
        let rows = build_rows(&cabinet, &items, &fields);
        total += rows.len().saturating_sub(1);
        common::append_cabinet_rows(&mut all_rows, &mut header_done, rows);
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк товаров на лист «{sheet_name}»"),
    })
}

async fn fetch_all(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    cabinet: &str,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut last_id = String::new();
    let mut page = 0u32;

    loop {
        page += 1;
        let body = json!({
            "filter": { "visibility": "ALL" },
            "last_id": last_id,
            "limit": PAGE_LIMIT,
        });
        let profile = crate::core::rate_profiles::ozon::product_list::profile();
        let v = crate::scripts::ozon_util::ozon_post(
            ctx,
            cabinet,
            crate::core::rate_profiles::ozon::product_list::SCOPE,
            &profile,
            PATH,
            client_id,
            api_key,
            &body,
        )
        .await?;

        let result = v.get("result").cloned().unwrap_or(json!({}));
        let items = result
            .get("items")
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let n = items.len();
        let next = result
            .get("last_id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        ctx.log
            .info(format!("Кабинет «{cabinet}»: страница {page}, items={n}"));
        all.extend(items);
        if next.is_empty() || n == 0 {
            break;
        }
        last_id = next;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Ok(all)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_product_list.cabinet"), true),
        ScriptField::new("offer_id", crate::core::i18n::t("fields.ozon_product_list.offer_id"), true),
        ScriptField::new("product_id", crate::core::i18n::t("fields.ozon_product_list.product_id"), true),
        ScriptField::new("archived", crate::core::i18n::t("fields.ozon_product_list.archived"), true),
        ScriptField::new("has_fbo_stocks", crate::core::i18n::t("fields.ozon_product_list.has_fbo_stocks"), true),
        ScriptField::new("has_fbs_stocks", crate::core::i18n::t("fields.ozon_product_list.has_fbs_stocks"), true),
        ScriptField::new("is_discounted", crate::core::i18n::t("fields.ozon_product_list.is_discounted"), true),
        ScriptField::new("quant_code", crate::core::i18n::t("fields.ozon_product_list.quant_code"), true),
        ScriptField::new("quant_size", crate::core::i18n::t("fields.ozon_product_list.quant_size"), true),
    ]
}

fn yes_no(v: Option<bool>) -> String {
    if v == Some(true) {
        "Да".into()
    } else {
        "Нет".into()
    }
}

fn cell(map: &std::collections::HashMap<&str, String>, name: &str) -> String {
    map.get(name).cloned().unwrap_or_default()
}

pub fn build_rows(cabinet: &str, items: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = vec![selected.iter().map(|f| f.title.clone()).collect()];

    for item in items {
        let base = |quant_code: String, quant_size: String| {
            let mut map = std::collections::HashMap::new();
            map.insert("cabinet", cabinet.to_string());
            map.insert(
                "offer_id",
                item.get("offer_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            );
            map.insert(
                "product_id",
                item.get("product_id")
                    .and_then(|x| x.as_i64().or_else(|| x.as_u64().map(|n| n as i64)))
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            );
            map.insert("archived", yes_no(item.get("archived").and_then(|x| x.as_bool())));
            map.insert(
                "has_fbo_stocks",
                yes_no(item.get("has_fbo_stocks").and_then(|x| x.as_bool())),
            );
            map.insert(
                "has_fbs_stocks",
                yes_no(item.get("has_fbs_stocks").and_then(|x| x.as_bool())),
            );
            map.insert(
                "is_discounted",
                yes_no(item.get("is_discounted").and_then(|x| x.as_bool())),
            );
            map.insert("quant_code", quant_code);
            map.insert("quant_size", quant_size);
            map
        };

        let quants = item
            .get("quants")
            .and_then(|q| q.as_array())
            .cloned()
            .unwrap_or_default();
        if quants.is_empty() {
            let map = base(String::new(), String::new());
            rows.push(selected.iter().map(|f| cell(&map, &f.name)).collect());
        } else {
            for q in &quants {
                let code = q
                    .get("quant_code")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let size = q
                    .get("quant_size")
                    .and_then(|x| x.as_i64().or_else(|| x.as_u64().map(|n| n as i64)))
                    .map(|n| n.to_string())
                    .unwrap_or_default();
                let map = base(code, size);
                rows.push(selected.iter().map(|f| cell(&map, &f.name)).collect());
            }
        }
    }
    rows
}
