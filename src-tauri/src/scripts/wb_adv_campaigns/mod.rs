pub mod fields;

use crate::core::rate_profiles::adv_campaigns_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use self::fields::{build_rows, default_fields};
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const ADV_COUNT_URL: &str = "https://advert-api.wildberries.ru/adv/v1/promotion/count";

/// Список рекламных кампаний WB (`adv_campaigns_list`) → Google Sheets.
pub struct AdvCampaignsListScript;

impl Script for AdvCampaignsListScript {
    fn key(&self) -> &'static str {
        "adv_campaigns_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.adv_campaigns_list")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_adv_campaigns(ctx).await })
    }
}

async fn run_adv_campaigns(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Список рекламных кампаний»: старт");

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
                    return Ok((cabinet, None));
                }
                let (all, groups) = fetch_adv_count(ctx, &cabinet, token.trim()).await?;
                Ok((cabinet, Some((all, groups))))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, part) in parts {
        let Some((all, groups)) = part else {
            continue;
        };
        let rows = build_rows(&cabinet, all, &groups, &fields);
        let n = rows.len().saturating_sub(1);
        ctx.log
            .info(format!("Кабинет «{cabinet}»: {n} кампаний (all={all})"));
        total += n;
        common::append_cabinet_rows(&mut all_rows, &mut header_done, rows);
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} кампаний на лист «{sheet_name}»"),
    })
}

async fn fetch_adv_count(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<(i64, Vec<Value>), String> {
    ctx.log.info("GET adv/v1/promotion/count");
    let profile = rate::profile();
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            ADV_COUNT_URL,
            Auth::Raw(token),
        )
        .await?;
    let all = v.get("all").and_then(|x| x.as_i64()).unwrap_or(0);
    let groups = v
        .get("adverts")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    Ok((all, groups))
}
