use crate::core::rate_profiles::tariffs_box as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use crate::scripts::common;
use chrono::NaiveDate;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const BOX_URL: &str = "https://common-api.wildberries.ru/api/v1/tariffs/box";

/// Тарифы коробов WB (`tariffs_box`) → Google Sheets.
pub struct BoxTariffsScript;

impl Script for BoxTariffsScript {
    fn key(&self) -> &'static str {
        "tariffs_box"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.tariffs_box")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_box_tariffs(ctx).await })
    }
}

async fn run_box_tariffs(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Тарифы для коробов»: старт");

    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    ctx.log
        .info(format!("Выбраны кабинеты: {}", cabinets.join(", ")));

    let date = resolve_date(ctx)?;
    let date_api = date.format("%Y-%m-%d").to_string();
    let date_display = sheet_dates::format_date(date);
    ctx.log.info(format!("Параметры: date={date_api}"));

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
            let date_api = date_api.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, None));
                }
                let (dt_next, dt_till, warehouses) =
                    fetch_box_tariffs(ctx, &cabinet, token.trim(), &date_api).await?;
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: {} складов",
                    warehouses.len()
                ));
                Ok((cabinet, Some((dt_next, dt_till, warehouses))))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, part) in parts {
        let Some((dt_next, dt_till, warehouses)) = part else {
            continue;
        };
        if warehouses.is_empty() {
            continue;
        }
        total += warehouses.len();
        let meta = BoxMeta {
            cabinet: &cabinet,
            date_display: &date_display,
            dt_next_box: &dt_next,
            dt_till_max: &dt_till,
        };
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&meta, &warehouses, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} складов на лист «{sheet_name}»"),
    })
}

fn resolve_date(ctx: &ScriptContext) -> Result<NaiveDate, String> {
    crate::scripts::dates::resolve_single_date(ctx)
}

async fn fetch_box_tariffs(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    date_api: &str,
) -> Result<(String, String, Vec<Value>), String> {
    let url = format!("{BOX_URL}?date={date_api}");
    ctx.log.info(format!("GET tariffs/box date={date_api}"));
    let profile = rate::profile();

    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile: &profile,
            },
            &url,
            Auth::Raw(token),
        )
        .await?;

    let data = v
        .get("data")
        .or_else(|| v.pointer("/response/data"))
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let dt_next = data
        .get("dtNextBox")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let dt_till = data
        .get("dtTillMax")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let warehouses = data
        .get("warehouseList")
        .and_then(|w| w.as_array())
        .cloned()
        .unwrap_or_default();

    Ok((dt_next, dt_till, warehouses))
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("date", true),
        field("dtNextBox", true),
        field("dtTillMax", true),
        field("geoName", true),
        field("warehouseName", true),
        field("boxDeliveryBase", true),
        field("boxDeliveryLiter", true),
        field("boxDeliveryMarketplaceBase", true),
        field("boxDeliveryMarketplaceLiter", true),
        field("boxStorageBase", true),
        field("boxStorageLiter", true),
        field("boxDeliveryCoefExpr", false),
        field("boxDeliveryMarketplaceCoefExpr", false),
        field("boxStorageCoefExpr", false),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.tariffs_box.{name}")),
        selected,
    }
}

pub struct BoxMeta<'a> {
    pub cabinet: &'a str,
    pub date_display: &'a str,
    pub dt_next_box: &'a str,
    pub dt_till_max: &'a str,
}

pub fn build_rows(meta: &BoxMeta<'_>, warehouses: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = Vec::with_capacity(warehouses.len() + 1);
    rows.push(selected.iter().map(|f| f.title.clone()).collect());
    for w in warehouses {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), meta, w));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, meta: &BoxMeta<'_>, w: &Value) -> String {
    match name {
        "cabinet" => meta.cabinet.to_string(),
        "date" => meta.date_display.to_string(),
        "dtNextBox" => meta.dt_next_box.to_string(),
        "dtTillMax" => meta.dt_till_max.to_string(),
        "geoName" => str(w, "geoName"),
        "warehouseName" => str(w, "warehouseName"),
        "boxDeliveryBase" => str(w, "boxDeliveryBase"),
        "boxDeliveryLiter" => str(w, "boxDeliveryLiter"),
        "boxDeliveryMarketplaceBase" => str(w, "boxDeliveryMarketplaceBase"),
        "boxDeliveryMarketplaceLiter" => str(w, "boxDeliveryMarketplaceLiter"),
        "boxStorageBase" => str(w, "boxStorageBase"),
        "boxStorageLiter" => str(w, "boxStorageLiter"),
        "boxDeliveryCoefExpr" => str(w, "boxDeliveryCoefExpr"),
        "boxDeliveryMarketplaceCoefExpr" => str(w, "boxDeliveryMarketplaceCoefExpr"),
        "boxStorageCoefExpr" => str(w, "boxStorageCoefExpr"),
        _ => String::new(),
    }
}

fn str(it: &Value, key: &str) -> String {
    match it.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(v) => v.as_str().unwrap_or(&v.to_string()).to_string(),
    }
}
