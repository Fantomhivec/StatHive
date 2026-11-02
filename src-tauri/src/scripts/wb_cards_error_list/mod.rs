//! `cards_error_list` — черновики / ошибки карточек WB → Sheets.

use crate::core::rate_profiles::cards_error_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://content-api.wildberries.ru/content/v2/cards/error/list?locale=ru";

pub struct CardsErrorListScript;

impl Script for CardsErrorListScript {
    fn key(&self) -> &'static str {
        "cards_error_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cards_error_list")
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
    ctx.log.info("Скрипт «Черновики карточек»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<_> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Err("Поля выгрузки не выбраны".into());
    }
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let items = fetch_all(ctx, &cabinet, token.trim()).await?;
                Ok((cabinet, items))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, items) in parts {
        if items.is_empty() {
            continue;
        }
        let mut rows_cab = 0usize;
        for item in &items {
            let batch = item
                .get("batchUUID")
                .or_else(|| item.get("batchUuid"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let vendor_codes = item
                .get("vendorCodes")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for vc in vendor_codes {
                let code = match vc.as_str() {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => continue,
                };
                let mut row = Vec::with_capacity(selected.len());
                for f in &selected {
                    row.push(cell_value(&cabinet, item, batch, code, &f.name));
                }
                all_rows.push(row);
                rows_cab += 1;
            }
        }
        total += rows_cab;
        ctx.log.info(format!(
            "Кабинет «{cabinet}»: пакетов={}, строк={rows_cab}",
            items.len()
        ));
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк черновиков на лист «{sheet_name}»"),
    })
}

fn cell_value(cabinet: &str, item: &Value, batch: &str, vendor: &str, name: &str) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "batchUUID" => batch.to_string(),
        "vendorCode" => vendor.to_string(),
        "subjectId" => map_field(item, "subjects", vendor, "id"),
        "subjectName" => map_field(item, "subjects", vendor, "name"),
        "brandId" => map_field(item, "brands", vendor, "id"),
        "brandName" => map_field(item, "brands", vendor, "name"),
        "errors" => {
            let Some(errs) = item
                .get("errors")
                .and_then(|e| e.get(vendor))
                .and_then(|a| a.as_array())
            else {
                return String::new();
            };
            errs.iter()
                .filter_map(|e| e.as_str())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" | ")
        }
        _ => String::new(),
    }
}

fn map_field(item: &Value, map_key: &str, vendor: &str, field: &str) -> String {
    let Some(obj) = item.get(map_key).and_then(|m| m.get(vendor)) else {
        return String::new();
    };
    match obj.get(field) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

async fn fetch_all(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut cursor_updated: Option<String> = None;
    let mut cursor_batch: Option<String> = None;
    let mut page = 0u32;
    let profile = rate::profile();

    loop {
        page += 1;
        let mut cursor = serde_json::json!({ "limit": 100 });
        if let Some(ref u) = cursor_updated {
            cursor["updatedAt"] = Value::String(u.clone());
        }
        if let Some(ref b) = cursor_batch {
            cursor["batchUUID"] = Value::String(b.clone());
        }
        let body = serde_json::json!({
            "cursor": cursor,
            "order": { "ascending": true }
        });
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                URL,
                Auth::Bearer(token),
                &body,
            )
            .await?;
        let data = v.get("data");
        let items = data
            .and_then(|d| d.get("items"))
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        all.extend(items);

        let cursor = data.and_then(|d| d.get("cursor"));
        let next = cursor
            .and_then(|c| c.get("next"))
            .and_then(|n| n.as_bool())
            .unwrap_or(false);
        if !next {
            ctx.log
                .info(format!("pagination complete, pages={page}, items={}", all.len()));
            break;
        }
        cursor_updated = cursor
            .and_then(|c| c.get("updatedAt"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        cursor_batch = cursor
            .and_then(|c| c.get("batchUUID"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        if cursor_updated.is_none() && cursor_batch.is_none() {
            break;
        }
    }
    Ok(all)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.cards_error_list.cabinet"), true),
        ScriptField::new("batchUUID", crate::core::i18n::t("fields.cards_error_list.batchUUID"), true),
        ScriptField::new("vendorCode", crate::core::i18n::t("fields.cards_error_list.vendorCode"), true),
        ScriptField::new("subjectId", crate::core::i18n::t("fields.cards_error_list.subjectId"), true),
        ScriptField::new("subjectName", crate::core::i18n::t("fields.cards_error_list.subjectName"), true),
        ScriptField::new("brandId", crate::core::i18n::t("fields.cards_error_list.brandId"), true),
        ScriptField::new("brandName", crate::core::i18n::t("fields.cards_error_list.brandName"), true),
        ScriptField::new("errors", crate::core::i18n::t("fields.cards_error_list.errors"), true),
    ]
}
