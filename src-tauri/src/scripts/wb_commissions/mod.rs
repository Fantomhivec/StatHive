use crate::core::rate_profiles::tariffs_commission as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const COMMISSION_URL: &str = "https://common-api.wildberries.ru/api/v1/tariffs/commission";
const LOCALE: &str = "ru";

/// Комиссии по категориям WB (`tariffs_commission`) → Google Sheets.
pub struct CommissionsScript;

impl Script for CommissionsScript {
    fn key(&self) -> &'static str {
        "tariffs_commission"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.tariffs_commission")
    }

    fn default_fields(&self) -> Vec<ScriptField> {
        default_fields()
    }

    fn execute<'a>(
        &'a self,
        ctx: &'a ScriptContext,
    ) -> Pin<Box<dyn Future<Output = ScriptResult> + Send + 'a>> {
        Box::pin(async move { run_commissions(ctx).await })
    }
}

async fn run_commissions(ctx: &ScriptContext) -> ScriptResult {
    ctx.log.info("Скрипт «Комиссии по категориям»: старт");

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
                    return Ok((cabinet, Vec::new()));
                }
                let report = fetch_commissions(ctx, &cabinet, token.trim()).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: {} записей", report.len()));
                Ok((cabinet, report))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = Vec::new();
    let mut header_done = false;
    let mut total = 0usize;
    for (cabinet, report) in parts {
        if report.is_empty() {
            continue;
        }
        total += report.len();
        common::append_cabinet_rows(
            &mut all_rows,
            &mut header_done,
            build_rows(&cabinet, LOCALE, &report, &fields),
        );
    }

    if all_rows.is_empty() {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;

    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} комиссий на лист «{sheet_name}»"),
    })
}

async fn fetch_commissions(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<Value>, String> {
    let url = format!("{COMMISSION_URL}?locale={LOCALE}");
    ctx.log.info(format!("GET tariffs/commission locale={LOCALE}"));
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

    let report = v
        .get("report")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(report)
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        field("cabinet", true),
        field("locale", true),
        field("parentID", true),
        field("parentName", true),
        field("subjectID", true),
        field("subjectName", true),
        field("kgvpBooking", true),
        field("kgvpMarketplace", true),
        field("kgvpPickup", true),
        field("kgvpSupplier", true),
        field("kgvpSupplierExpress", true),
        field("paidStorageKgvp", true),
    ]
}

fn field(name: &str, selected: bool) -> ScriptField {
    ScriptField {
        name: name.into(),
        title: crate::core::i18n::t(&format!("fields.tariffs_commission.{name}")),
        selected,
    }
}

pub fn build_rows(cabinet: &str, locale: &str, report: &[Value], fields: &[ScriptField]) -> Vec<Vec<String>> {
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let mut rows = Vec::with_capacity(report.len() + 1);
    rows.push(selected.iter().map(|f| f.title.clone()).collect());
    for it in report {
        let mut row = Vec::with_capacity(selected.len());
        for f in &selected {
            row.push(cell(f.name.as_str(), cabinet, locale, it));
        }
        rows.push(row);
    }
    rows
}

fn cell(name: &str, cabinet: &str, locale: &str, it: &Value) -> String {
    match name {
        "cabinet" => cabinet.to_string(),
        "locale" => locale.to_string(),
        // ID как текст (префикс '), чтобы Sheets не делал дату.
        "parentID" => id_text(it, "parentID"),
        "parentName" => str(it, "parentName"),
        "subjectID" => id_text(it, "subjectID"),
        "subjectName" => str(it, "subjectName"),
        "kgvpBooking" => num(it, "kgvpBooking"),
        "kgvpMarketplace" => num(it, "kgvpMarketplace"),
        "kgvpPickup" => num(it, "kgvpPickup"),
        "kgvpSupplier" => num(it, "kgvpSupplier"),
        "kgvpSupplierExpress" => num(it, "kgvpSupplierExpress"),
        "paidStorageKgvp" => num(it, "paidStorageKgvp"),
        _ => String::new(),
    }
}

fn id_text(it: &Value, key: &str) -> String {
    match it.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => format!("'{n}"),
        Some(v) => {
            let s = v.as_str().unwrap_or(&v.to_string()).trim().to_string();
            if s.is_empty() {
                String::new()
            } else if s.starts_with('\'') {
                s
            } else {
                format!("'{s}")
            }
        }
    }
}

fn str(it: &Value, key: &str) -> String {
    it.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn num(it: &Value, key: &str) -> String {
    match it.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
    }
}
