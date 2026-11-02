pub mod docs;
pub mod fields;

use crate::core::rate_profiles::cards_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use self::fields::{build_rows, default_fields};
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const CARDS_URL: &str = "https://content-api.wildberries.ru/content/v2/get/cards/list";
const PAGE_LIMIT: i64 = 100;

/// Выгрузка карточек WB (`cards_list`) → Google Sheets.
pub struct CardsScript;

impl Script for CardsScript {
    fn key(&self) -> &'static str {
        "cards_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.cards_list")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_cards(ctx).await })
    }
}

async fn run_cards(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Карточки WB»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log.info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
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
                let cards = fetch_all_cards(ctx, &cabinet, token.trim()).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} карточек", cards.len()));
                Ok((cabinet, cards))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total_cards = 0usize;
    for (cabinet, cards) in parts {
        if cards.is_empty() {
            continue;
        }
        total_cards += cards.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, &cards, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total_cards,
        message: format!("Записано {total_cards} карточек на лист «{sheet_name}»"),
    })
}

async fn fetch_all_cards(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut cursor_updated_at: Option<Value> = None;
    let mut cursor_nm_id: Option<i64> = None;
    let mut page = 0u32;
    let profile = rate::profile();

    loop {
        page += 1;
        let mut cursor = serde_json::json!({ "limit": PAGE_LIMIT });
        if let Some(ref ua) = cursor_updated_at {
            cursor["updatedAt"] = ua.clone();
        }
        if let Some(nm) = cursor_nm_id {
            cursor["nmID"] = Value::from(nm);
        }
        let body = serde_json::json!({
            "settings": {
                "cursor": cursor,
                "filter": { "withPhoto": -1 }
            }
        });

        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                CARDS_URL,
                Auth::Bearer(token),
                &body,
            )
            .await?;
        // Забрать cards без clone всего массива Value.
        let mut v = v;
        let batch = match v.get_mut("cards").map(std::mem::take) {
            Some(Value::Array(a)) => a,
            _ => Vec::new(),
        };
        let batch_len = batch.len();
        ctx.log.info(format!("Страница {page}: {batch_len} карт"));
        all.extend(batch);

        let resp_cursor = v.get("cursor");
        let total = resp_cursor
            .and_then(|c| c.get("total"))
            .and_then(|t| t.as_i64())
            .unwrap_or(batch_len as i64);

        cursor_updated_at = resp_cursor.and_then(|c| c.get("updatedAt")).cloned();
        cursor_nm_id = resp_cursor
            .and_then(|c| c.get("nmID"))
            .and_then(|n| n.as_i64());

        if batch_len == 0 || batch_len < PAGE_LIMIT as usize || total < PAGE_LIMIT {
            break;
        }
        if cursor_updated_at.is_none() && cursor_nm_id.is_none() {
            break;
        }
    }

    Ok(all)
}
