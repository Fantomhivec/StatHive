//! OZON: Среднее время доставки — POST /v1/analytics/average-delivery-time/summary.
//! Паритет `OZ_AverageDeliveryTimeScript`.

use serde_json::Value;
use crate::core::rate_profiles::ozon::average_delivery_time as rate;
use crate::core::script::{Script, ScriptContext, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::ozon_util::ozon_post;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;

const PATH: &str = "/v1/analytics/average-delivery-time/summary";

pub struct OzonAverageDeliveryTimeScript;

impl Script for OzonAverageDeliveryTimeScript {
    fn key(&self) -> &'static str {
        "ozon_average_delivery_time"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_average_delivery_time")
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
    ctx.log
        .info("Скрипт «OZON: Среднее время доставки»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }

    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };

    let fields = common::resolve_fields(ctx, default_fields());
    let pairs = common::load_ozon_pairs()?;
    let profile = rate::profile();
    let body = json!({});
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            let profile = profile.clone();
            let body = body.clone();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log
                        .warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, None));
                };

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
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: данные получены"));
                Ok((cabinet, Some(v)))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, v) in parts {
        let Some(v) = v else {
            continue;
        };
        let root = v.get("result").unwrap_or(&v);
        let Some(row) = build_row(root, &cabinet, &fields) else {
            ctx.log
                .warn(format!("Кабинет «{cabinet}»: неожиданный формат ответа"));
            continue;
        };
        common::ensure_header_row(&mut all_rows, &mut header_done, &fields);
        all_rows.push(row);
        total += 1;
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(crate::core::script::ScriptOutput {
        rows_written: total,
        message: format!(
            "Записано {total} строк среднего времени доставки на лист «{sheet_name}»"
        ),
    })
}

// --- fields (merged) ---
use crate::scripts::ozon_util::{f2_ru, format_ozon_dt, json_str};
use std::collections::HashMap;

pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.ozon_average_delivery_time.cabinet"), true),
        ScriptField::new("average_delivery_time", crate::core::i18n::t("fields.ozon_average_delivery_time.average_delivery_time"), true),
        ScriptField::new(
            "perfect_delivery_time",
            "Рекомендуемое среднее время доставки",
            true,
        ),
        ScriptField::new(
            "tariff_start",
            "Значение метрики среднего времени доставки",
            true,
        ),
        ScriptField::new("tariff_status", crate::core::i18n::t("fields.ozon_average_delivery_time.tariff_status"), true),
        ScriptField::new(
            "tariff_value",
            "Процент к базовому тарифу логистики FBO",
            true,
        ),
        ScriptField::new("tariff_fee", crate::core::i18n::t("fields.ozon_average_delivery_time.tariff_fee"), true),
        ScriptField::new("lost_profit", crate::core::i18n::t("fields.ozon_average_delivery_time.lost_profit"), true),
        ScriptField::new(
            "updated_at",
            "Дата и время последнего обновления",
            true,
        ),
    ]
}

pub fn build_row(
    response: &Value,
    cabinet: &str,
    fields: &[ScriptField],
) -> Option<Vec<String>> {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert("cabinet".into(), cabinet.to_string());
    map.insert(
        "average_delivery_time".into(),
        json_str(response.get("average_delivery_time")),
    );
    map.insert(
        "perfect_delivery_time".into(),
        json_str(response.get("perfect_delivery_time")),
    );

    if let Some(t) = response.get("current_tariff") {
        map.insert("tariff_start".into(), json_str(t.get("start")));
        map.insert("tariff_status".into(), json_str(t.get("tariff_status")));
        map.insert("tariff_value".into(), f2_ru(t.get("tariff_value")));
        map.insert("tariff_fee".into(), f2_ru(t.get("fee")));
    } else {
        map.insert("tariff_start".into(), String::new());
        map.insert("tariff_status".into(), String::new());
        map.insert("tariff_value".into(), String::new());
        map.insert("tariff_fee".into(), String::new());
    }

    map.insert("lost_profit".into(), f2_ru(response.get("lost_profit")));

    let updated_raw = json_str(response.get("updated_at"));
    let updated = if updated_raw.is_empty() || updated_raw == "0001-01-01T00:00:00Z" {
        String::new()
    } else {
        format_ozon_dt(&updated_raw)
    };
    map.insert("updated_at".into(), updated);

    Some(
        fields
            .iter()
            .filter(|f| f.selected)
            .map(|f| map.get(&f.name).cloned().unwrap_or_default())
            .collect(),
    )
}
