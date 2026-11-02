//! `transit_tariffs` — GET supplies-api `/api/v1/transit-tariffs` (raw token).
//! В строку попадает только `boxTariff[0]`; сортировка `activeFrom` по убыванию.

use crate::core::rate_profiles::transit_tariffs as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::core::sheet_dates;
use crate::scripts::common;
use chrono::{DateTime, NaiveDateTime};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::pin::Pin;

pub struct TransitTariffsScript;

impl Script for TransitTariffsScript {
    fn key(&self) -> &'static str {
        "transit_tariffs"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.transit_tariffs")
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

struct Built {
    sort: Option<i64>,
    cabinet: String,
    cells: Vec<String>,
}

async fn run(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Транзитные направления»: старт");
    let cabinets = common::resolve_cabinets(ctx);
    if cabinets.is_empty() {
        return Ok(common::empty_cabinets_output());
    }
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    if selected.is_empty() {
        return Err("Не выбраны поля для выгрузки".into());
    }
    let cab_idx = selected.iter().position(|f| f.name == "cabinet");
    let wb_tokens = common::load_wb_tokens()?;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                match fetch(ctx, &cabinet, token.trim(), &profile).await {
                    Ok(list) => {
                        ctx.log.info(format!(
                            "Кабинет «{cabinet}»: {} направлений",
                            list.len()
                        ));
                        Ok((cabinet, list))
                    }
                    Err(e) => {
                        ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                        Ok((cabinet, Vec::new()))
                    }
                }
            }
        },
    )
    .await?;

    let mut built = Vec::new();
    for (cabinet, list) in parts {
        for it in &list {
            let sort = active_ts(it.get("activeFrom"));
            let cells = selected
                .iter()
                .map(|f| cell(&f.name, &cabinet, it))
                .collect();
            built.push(Built {
                sort,
                cabinet: cabinet.clone(),
                cells,
            });
        }
    }
    if built.is_empty() {
        return Ok(common::empty_data_output());
    }
    let rows = collapse(&cabinets, cab_idx, built);
    let mut all_rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let total = rows.len();
    all_rows.extend(rows);
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано транзитных направлений: {total} → «{sheet_name}»"),
    })
}

fn collapse(cabinets: &[String], cab_idx: Option<usize>, built: Vec<Built>) -> Vec<Vec<String>> {
    let Some(idx) = cab_idx else {
        let mut rows = built;
        rows.sort_by(|a, b| b.sort.cmp(&a.sort));
        return rows.into_iter().map(|r| r.cells).collect();
    };
    let all: BTreeSet<String> = cabinets.iter().cloned().collect();
    let mut groups: HashMap<String, (Option<i64>, Vec<String>, BTreeSet<String>)> = HashMap::new();
    for row in built {
        let key = row
            .cells
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != idx)
            .map(|(_, c)| c.as_str())
            .collect::<Vec<_>>()
            .join("\u{1f}");
        let key = format!("{:?}|{key}", row.sort);
        if let Some((_, _, cabs)) = groups.get_mut(&key) {
            cabs.insert(row.cabinet);
        } else {
            groups.insert(
                key,
                (row.sort, row.cells, BTreeSet::from([row.cabinet])),
            );
        }
    }
    let mut items: Vec<_> = groups.into_values().collect();
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items
        .into_iter()
        .map(|( _sort, mut cells, cabs)| {
            let label = if !all.is_empty() && cabs == all {
                "ВСЕ".to_string()
            } else {
                cabs.into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            if idx < cells.len() {
                cells[idx] = label;
            }
            cells
        })
        .collect()
}

async fn fetch(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
) -> Result<Vec<Value>, String> {
    let v = ctx
        .request
        .get_json(
            Limit {
                cabinet,
                scope: rate::SCOPE,
                profile,
            },
            "https://supplies-api.wildberries.ru/api/v1/transit-tariffs",
            Auth::Raw(token),
        )
        .await?;
    Ok(match v {
        Value::Array(a) => a,
        other => other
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default(),
    })
}

fn cell(name: &str, cabinet: &str, it: &Value) -> String {
    let box0 = it.get("boxTariff").and_then(|b| b.as_array()).and_then(|a| a.first());
    match name {
        "cabinet" => cabinet.to_string(),
        "transitWarehouseName" => scalar(it.get("transitWarehouseName")),
        "destinationWarehouseName" => scalar(it.get("destinationWarehouseName")),
        "activeFrom" => stamp(it.get("activeFrom")),
        "palletTariff" => scalar(it.get("palletTariff")),
        "boxTariffFrom" => scalar(box0.and_then(|b| b.get("from"))),
        "boxTariffTo" => scalar(box0.and_then(|b| b.get("to"))),
        "boxTariffValue" => f2(box0.and_then(|b| b.get("value"))),
        "boxTariffAvailable" => match it.get("boxTariff") {
            None | Some(Value::Null) => "Нет".into(),
            Some(_) => "Да".into(),
        },
        _ => String::new(),
    }
}

fn active_ts(v: Option<&Value>) -> Option<i64> {
    let raw = scalar(v);
    if raw.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(&raw) {
        return Some(dt.timestamp());
    }
    NaiveDateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|dt| dt.and_utc().timestamp())
}

fn stamp(v: Option<&Value>) -> String {
    let raw = scalar(v);
    if raw.is_empty() {
        String::new()
    } else {
        sheet_dates::format_wb_timestamp(&raw)
    }
}

fn f2(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => n
            .as_f64()
            .map(|f| format!("{f:.2}"))
            .unwrap_or_else(|| n.to_string()),
        other => scalar(other),
    }
}

fn scalar(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

fn default_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "transitWarehouseName",
        "destinationWarehouseName",
        "activeFrom",
        "palletTariff",
        "boxTariffFrom",
        "boxTariffTo",
        "boxTariffValue",
        "boxTariffAvailable",
    ]
    .into_iter()
    .map(|n| ScriptField {
        name: n.into(),
        title: crate::core::i18n::t(&format!("fields.transit_tariffs.{n}")),
        selected: true,
    })
    .collect()
}
