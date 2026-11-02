//! `acceptance_options` — POST supplies-api `/api/v1/acceptance/options`.
//! Баркоды — `nm_id_source_range`, количество — `stocks_skus_source_range`.

use crate::core::rate_profiles::{acceptance_options as rate, supplies_warehouses as wh_rate};
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::wb_supplies::api::fbo_warehouses;
use crate::scripts::{common, sheet_ranges};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

const BATCH: usize = 5000;

pub struct AcceptanceOptionsScript;

impl Script for AcceptanceOptionsScript {
    fn key(&self) -> &'static str {
        "acceptance_options"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.acceptance_options")
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
    ctx.log.info("Скрипт «Опции приёмки»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let (src_id, src_sheet) = source_sheet(ctx)?;
    let cab_range = param(ctx, "cabinets_source_range");
    let barcode_range = first_nonempty(&[
        param(ctx, "barcodes_source_range"),
        param(ctx, "nm_id_source_range"),
    ]);
    let qty_range = first_nonempty(&[
        param(ctx, "quantities_source_range"),
        param(ctx, "stocks_skus_source_range"),
    ]);
    if cab_range.is_empty() || barcode_range.is_empty() || qty_range.is_empty() {
        return Err(
            "Укажите диапазоны: кабинеты, баркоды (nmID) и количества (chrtId)".into(),
        );
    }
    let client = common::google_sheets_client(ctx).await?;
    let cols = sheet_ranges::read_first_cols(
        &client,
        &src_id,
        &src_sheet,
        &[cab_range, barcode_range, qty_range],
    )
    .await?;
    let grouped = group_rows(&cols);
    if grouped.is_empty() {
        return Err("После фильтрации не осталось пар баркод/количество".into());
    }
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let flag_names = ["canBox", "canMonopallet", "canSupersafe", "isBoxOnPallet"];
    let flags: Vec<&str> = flag_names
        .into_iter()
        .filter(|n| selected.iter().any(|f| f.name == *n))
        .collect();

    let wb_tokens = common::load_wb_tokens()?;
    let cabinets: Vec<String> = grouped.keys().cloned().collect();
    let names = load_warehouse_names(ctx, &wb_tokens, &cabinets).await;
    let profile = rate::profile();
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let items = grouped.get(&cabinet).cloned().unwrap_or_default();
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let profile = profile.clone();
            let names = names.clone();
            let flags = flags.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log.warn(format!("Кабинет «{cabinet}»: нет токена"));
                    return Ok(Vec::new());
                }
                match cabinet_rows(ctx, &cabinet, token.trim(), &profile, &items, &names, &flags)
                    .await
                {
                    Ok(rows) => {
                        ctx.log
                            .info(format!("Кабинет «{cabinet}»: {} строк", rows.len()));
                        Ok(rows)
                    }
                    Err(e) => {
                        ctx.log.warn(format!("Кабинет «{cabinet}»: {e}"));
                        Ok(Vec::new())
                    }
                }
            }
        },
    )
    .await?;

    let mut all_rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for chunk in parts {
        for item in chunk {
            let row = selected
                .iter()
                .map(|f| item_cell(&f.name, &item))
                .collect();
            all_rows.push(row);
            total += 1;
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано опций приёмки: {total} → «{sheet_name}»"),
    })
}

struct OutRow {
    cabinet: String,
    barcode: String,
    quantity: i64,
    warehouse_id: String,
    warehouse_name: String,
    can_box: String,
    can_mono: String,
    can_safe: String,
    box_on_pallet: String,
    error: String,
}

fn item_cell(name: &str, row: &OutRow) -> String {
    match name {
        "cabinet" => row.cabinet.clone(),
        "barcode" => row.barcode.clone(),
        "quantity" => row.quantity.to_string(),
        "warehouseID" => row.warehouse_id.clone(),
        "warehouseName" => row.warehouse_name.clone(),
        "canBox" => row.can_box.clone(),
        "canMonopallet" => row.can_mono.clone(),
        "canSupersafe" => row.can_safe.clone(),
        "isBoxOnPallet" => row.box_on_pallet.clone(),
        "error" => row.error.clone(),
        _ => String::new(),
    }
}

async fn cabinet_rows(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    profile: &crate::core::rate_limit::RateLimitProfile,
    items: &[(String, i64)],
    names: &HashMap<i64, String>,
    flags: &[&str],
) -> Result<Vec<OutRow>, String> {
    let mut out = Vec::new();
    for batch in items.chunks(BATCH) {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let body = Value::Array(
            batch
                .iter()
                .map(|(barcode, quantity)| json!({ "barcode": barcode, "quantity": quantity }))
                .collect(),
        );
        let v = ctx
            .request
            .post_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile,
                },
                "https://supplies-api.wildberries.ru/api/v1/acceptance/options",
                Auth::Raw(token),
                &body,
            )
            .await?;
        let results = v
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_else(|| v.as_array().cloned().unwrap_or_default());
        let mut by_barcode: HashMap<String, Vec<Value>> = HashMap::new();
        for item in results {
            let key = item
                .get("barcode")
                .and_then(|b| b.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if !key.is_empty() {
                by_barcode.entry(key).or_default().push(item);
            }
        }
        for (barcode, quantity) in batch {
            let key = barcode.to_ascii_lowercase();
            let Some(hits) = by_barcode.get(&key) else {
                ctx.log.warn(format!(
                    "Кабинет «{cabinet}»: нет результата для баркода {barcode}"
                ));
                continue;
            };
            for hit in hits {
                expand_hit(cabinet, barcode, *quantity, hit, names, flags, &mut out);
            }
        }
    }
    Ok(out)
}

fn expand_hit(
    cabinet: &str,
    barcode: &str,
    quantity: i64,
    hit: &Value,
    names: &HashMap<i64, String>,
    flags: &[&str],
    out: &mut Vec<OutRow>,
) {
    let is_error = hit.get("isError").and_then(|v| v.as_bool()).unwrap_or(false)
        || hit.get("error").map(|e| !e.is_null()).unwrap_or(false);
    if is_error {
        out.push(blank_row(
            cabinet,
            barcode,
            quantity,
            error_text(hit.get("error")),
        ));
        return;
    }
    let warehouses = hit
        .get("warehouses")
        .and_then(|w| w.as_array())
        .cloned()
        .unwrap_or_default();
    if warehouses.is_empty() {
        out.push(blank_row(
            cabinet,
            barcode,
            quantity,
            "Нет доступных складов".into(),
        ));
        return;
    }
    for wh in warehouses {
        if !flags.is_empty() && !flags.iter().any(|f| wh.get(*f).and_then(|v| v.as_bool()).unwrap_or(false))
        {
            continue;
        }
        let id = wh
            .get("warehouseID")
            .or_else(|| wh.get("warehouseId"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        out.push(OutRow {
            cabinet: cabinet.to_string(),
            barcode: barcode.to_string(),
            quantity,
            warehouse_id: if id == 0 { String::new() } else { id.to_string() },
            warehouse_name: names.get(&id).cloned().unwrap_or_default(),
            can_box: bool_cell(wh.get("canBox")),
            can_mono: bool_cell(wh.get("canMonopallet")),
            can_safe: bool_cell(wh.get("canSupersafe")),
            box_on_pallet: bool_cell(wh.get("isBoxOnPallet")),
            error: String::new(),
        });
    }
}

fn blank_row(cabinet: &str, barcode: &str, quantity: i64, error: String) -> OutRow {
    OutRow {
        cabinet: cabinet.to_string(),
        barcode: barcode.to_string(),
        quantity,
        warehouse_id: String::new(),
        warehouse_name: String::new(),
        can_box: String::new(),
        can_mono: String::new(),
        can_safe: String::new(),
        box_on_pallet: String::new(),
        error,
    }
}

fn error_text(v: Option<&Value>) -> String {
    let Some(e) = v else {
        return "Ошибка".into();
    };
    let detail = e.get("detail").and_then(|x| x.as_str()).unwrap_or("").trim();
    if !detail.is_empty() {
        return detail.to_string();
    }
    let title = e.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
    if !title.is_empty() {
        return title.to_string();
    }
    "Ошибка".into()
}

fn bool_cell(v: Option<&Value>) -> String {
    match v.and_then(|x| x.as_bool()) {
        Some(true) => "true".into(),
        Some(false) => "false".into(),
        None => String::new(),
    }
}

fn group_rows(cols: &[Vec<String>]) -> HashMap<String, Vec<(String, i64)>> {
    let cabs = cols.first().cloned().unwrap_or_default();
    let bars = cols.get(1).cloned().unwrap_or_default();
    let qtys = cols.get(2).cloned().unwrap_or_default();
    let n = cabs.len().max(bars.len()).max(qtys.len());
    let mut grouped: HashMap<String, Vec<(String, i64)>> = HashMap::new();
    for i in 0..n {
        let cab = sheet_ranges::normalize_cabinet_name(cabs.get(i).map(|s| s.as_str()).unwrap_or(""));
        let barcode = bars.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
        let qty_raw = qtys.get(i).map(|s| s.as_str()).unwrap_or("");
        if cab.is_empty() && barcode.is_empty() && qty_raw.trim().is_empty() {
            continue;
        }
        let Some(qty) = sheet_ranges::parse_i64_cell(qty_raw).filter(|q| (1..=999_999).contains(q)) else {
            continue;
        };
        if cab.is_empty() || barcode.is_empty() {
            continue;
        }
        grouped.entry(cab).or_default().push((barcode, qty));
    }
    grouped
}

async fn load_warehouse_names(
    ctx: &ScriptContext,
    tokens: &common::WbTokenMap,
    cabinets: &[String],
) -> HashMap<i64, String> {
    let Some((cabinet, token)) = cabinets.iter().find_map(|c| {
        let t = common::resolve_wb_token_with_map(ctx, tokens, c);
        if t.trim().is_empty() {
            None
        } else {
            Some((c.clone(), t))
        }
    }) else {
        return HashMap::new();
    };
    let profile = wh_rate::profile();
    match fbo_warehouses(&ctx.request, &cabinet, token.trim(), wh_rate::SCOPE, &profile).await {
        Ok(list) => {
            let mut map = HashMap::new();
            for w in list {
                let id = w
                    .get("ID")
                    .or_else(|| w.get("id"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if id <= 0 {
                    continue;
                }
                let name = w.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                map.insert(id, name);
            }
            ctx.log
                .info(format!("Справочник складов поставок: {} записей", map.len()));
            map
        }
        Err(e) => {
            ctx.log.warn(format!("Справочник складов не загружен: {e}"));
            HashMap::new()
        }
    }
}

fn source_sheet(ctx: &ScriptContext) -> Result<(String, String), String> {
    let sid = first_nonempty(&[
        param(ctx, "second_spreadsheet_id"),
        param(ctx, "source_spreadsheet_id"),
        param(ctx, "spreadsheet_id"),
    ]);
    let sheet = first_nonempty(&[
        param(ctx, "second_sheet_name"),
        param(ctx, "source_sheet_name"),
        param(ctx, "sheet_name"),
    ]);
    if sid.is_empty() || sheet.is_empty() {
        return Err("Укажите вторую таблицу и лист с баркодами".into());
    }
    Ok((sid.to_string(), sheet.to_string()))
}

fn param<'a>(ctx: &'a ScriptContext, key: &str) -> &'a str {
    ctx.params.get(key).map(|s| s.trim()).unwrap_or("")
}

fn first_nonempty<'a>(parts: &[&'a str]) -> &'a str {
    parts.iter().copied().find(|s| !s.is_empty()).unwrap_or("")
}

fn default_fields() -> Vec<ScriptField> {
    [
        "cabinet",
        "barcode",
        "quantity",
        "warehouseID",
        "warehouseName",
        "canBox",
        "canMonopallet",
        "canSupersafe",
        "isBoxOnPallet",
        "error",
    ]
    .into_iter()
    .map(|n| ScriptField {
        name: n.into(),
        title: crate::core::i18n::t(&format!("fields.acceptance_options.{n}")),
        selected: true,
    })
    .collect()
}
