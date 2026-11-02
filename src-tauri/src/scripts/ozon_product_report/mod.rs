pub mod fields;

use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use self::fields::{default_fields, project_rows};
use crate::core::rate_profiles::ozon::product_report as rate;
use crate::core::request_sender::Limit;
use crate::scripts::ozon_util::ozon_post;
use calamine::{open_workbook_auto_from_rs, Data, Reader};
use serde_json::json;
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::time::Duration;

const CREATE_PATH: &str = "/v1/report/products/create";
const INFO_PATH: &str = "/v1/report/info";

pub struct OzonProductReportScript;

impl Script for OzonProductReportScript {
    fn key(&self) -> &'static str {
        "ozon_product_report"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.ozon_product_report")
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
    ctx.log.info("Скрипт «OZON: Отчёт по товарам»: старт");

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
        return Ok(ScriptOutput {
            rows_written: 0,
            message: "Не выбрано ни одного поля".into(),
        });
    }

    let pairs = common::load_ozon_pairs()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let keys = common::ozon_pair_from_map(&pairs, &cabinet).cloned();
            async move {
                let Some((client_id, api_key)) = keys else {
                    ctx.log.warn(format!("OZON ключи не заданы для «{cabinet}»"));
                    return Ok((cabinet, Vec::<Vec<String>>::new()));
                };

                ctx.log.info(format!("Кабинет «{cabinet}»: создание отчёта…"));
                let code = create_report(ctx, &client_id, &api_key, &cabinet).await?;
                ctx.log.info(format!("Кабинет «{cabinet}»: код отчёта {code}"));

                let file_url =
                    wait_report_url(ctx, &client_id, &api_key, &cabinet, &code).await?;
                ctx.log.info(format!("Кабинет «{cabinet}»: скачивание…"));

                let dl = crate::core::rate_profiles::ozon::download::profile();
                let bytes = ctx
                    .request
                    .download_bytes(
                        Limit {
                            cabinet: &cabinet,
                            scope: crate::core::rate_profiles::ozon::download::SCOPE,
                            profile: &dl,
                        },
                        &file_url,
                    )
                    .await?;
                let raw = parse_report_bytes(&bytes, &file_url)?;
                ctx.log.info(format!(
                    "Кабинет «{cabinet}»: разобрано {} строк (с шапкой)",
                    raw.len()
                ));
                Ok((cabinet, raw))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> =
        vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, raw) in parts {
        if raw.is_empty() {
            continue;
        }
        let rows = project_rows(&cabinet, &raw, &fields);
        total += rows.len();
        all_rows.extend(rows);
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }

    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} строк отчёта на лист «{sheet_name}»"),
    })
}

async fn create_report(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    cabinet: &str,
) -> Result<String, String> {
    let body = json!({
        "language": "DEFAULT",
        "offer_id": [],
        "search": "",
        "sku": [],
        "visibility": "ALL"
    });
    tokio::time::sleep(Duration::from_secs(1)).await;
    let profile = rate::profile();
    let v = ozon_post(
        ctx,
        cabinet,
        rate::SCOPE,
        &profile,
        CREATE_PATH,
        client_id,
        api_key,
        &body,
    )
    .await?;
    v.pointer("/result/code")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Не получен code отчёта".into())
}

async fn wait_report_url(
    ctx: &ScriptContext,
    client_id: &str,
    api_key: &str,
    cabinet: &str,
    code: &str,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err("Таймаут ожидания отчёта (10 мин)".into());
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
        let body = json!({ "code": code });
        let profile = rate::profile();
        let v = ozon_post(
            ctx,
            cabinet,
            rate::SCOPE,
            &profile,
            INFO_PATH,
            client_id,
            api_key,
            &body,
        )
        .await?;
        let result = v.get("result").cloned().unwrap_or(json!({}));
        if let Some(err) = result.get("error").and_then(|x| x.as_str()) {
            if !err.trim().is_empty() {
                return Err(format!("Ошибка генерации отчёта: {err}"));
            }
        }
        let status = result
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        ctx.log.info(format!("Кабинет «{cabinet}»: статус отчёта = {status}"));
        if status == "success" {
            return result
                .get("file")
                .and_then(|x| x.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| "Отчёт готов, но ссылка на файл пуста".into());
        }
        if status == "failed" {
            return Err("Генерация отчёта завершилась с ошибкой".into());
        }
    }
}

fn parse_report_bytes(bytes: &[u8], file_url: &str) -> Result<Vec<Vec<String>>, String> {
    if looks_like_csv(bytes, file_url) {
        return parse_csv(bytes);
    }
    match parse_excel(bytes) {
        Ok(rows) => Ok(rows),
        Err(_) => parse_csv(bytes),
    }
}

fn looks_like_csv(bytes: &[u8], file_url: &str) -> bool {
    if file_url.to_ascii_lowercase().contains(".csv") {
        return true;
    }
    if bytes.len() >= 2 && bytes[0] == 0x50 && bytes[1] == 0x4B {
        return false;
    }
    if bytes.len() >= 4 && bytes[0] == 0xD0 && bytes[1] == 0xCF {
        return false;
    }
    let sample_len = bytes.len().min(4096);
    let sample = String::from_utf8_lossy(&bytes[..sample_len]);
    sample.contains(';') || sample.contains(',') || sample.contains('\t')
}

fn parse_csv(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let text = decode_text(bytes);
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let cells = if line.contains(';') {
            split_csv_line(line, ';')
        } else if line.contains('\t') {
            split_csv_line(line, '\t')
        } else {
            split_csv_line(line, ',')
        };
        rows.push(cells);
    }
    Ok(rows)
}

fn decode_text(bytes: &[u8]) -> String {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn split_csv_line(line: &str, sep: char) -> Vec<String> {
    line.split(sep).map(|s| s.trim().trim_matches('"').to_string()).collect()
}

fn parse_excel(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook =
        open_workbook_auto_from_rs(cursor).map_err(|e| format!("Excel open: {e}"))?;
    let names = workbook.sheet_names().to_vec();
    let sheet = names.first().ok_or_else(|| "В книге нет листов".to_string())?;
    let range = workbook
        .worksheet_range(sheet)
        .map_err(|e| format!("Лист «{sheet}»: {e}"))?;
    let mut rows = Vec::new();
    for row in range.rows() {
        rows.push(row.iter().map(cell_to_string).collect());
    }
    Ok(rows)
}

fn cell_to_string(v: &Data) -> String {
    match v {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}
