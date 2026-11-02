//! Общие helpers запуска скриптов (кабинеты, поля, Sheets, токены).

use crate::core::env_keys;
use crate::core::script::{ScriptContext, ScriptOutput};
use crate::core::script_field::ScriptField;
use std::collections::HashMap;

/// Числовой nmId/SKU == 0 (`0`, `0.0`, `0,0`…). Нечисловые ключи — false.
/// Паритет `CalculatorArticleIndex.IsZeroNumericId`.

pub fn is_zero_numeric_id(raw: &str) -> bool {
    let k = raw
        .trim()
        .replace('\u{00a0}', " ")
        .replace(' ', "")
        .replace(',', ".");
    if k.is_empty() {
        return false;
    }
    k.parse::<f64>().ok().is_some_and(|n| n == 0.0)
}

/// Кабинеты из `cabinets` (через `,;`|) или одиночный `cabinet`.
pub fn resolve_cabinets(ctx: &ScriptContext) -> Vec<String> {
    if let Some(raw) = ctx.params.get("cabinets") {
        let list: Vec<String> = raw
            .split(|c| c == ',' || c == ';' || c == '|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    ctx.params
        .get("cabinet")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .into_iter()
        .collect()
}

/// Поля из `selected_fields_json` или defaults скрипта.
pub fn resolve_fields(ctx: &ScriptContext, defaults: Vec<ScriptField>) -> Vec<ScriptField> {
    if let Some(raw) = ctx.params.get("selected_fields_json") {
        if let Ok(list) = serde_json::from_str::<Vec<ScriptField>>(raw) {
            if !list.is_empty() {
                return list;
            }
        }
    }
    defaults
}

pub(crate) fn export_to_excel(ctx: &ScriptContext) -> bool {
    ctx.params
        .get("export_to_excel")
        .map(|s| {
            let t = s.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes"
        })
        .unwrap_or(false)
}

pub(crate) fn dry_run(ctx: &ScriptContext) -> bool {
    ctx.params
        .get("dry_run")
        .map(|s| {
            let t = s.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes"
        })
        .unwrap_or(false)
}

pub(crate) fn excel_file_path(ctx: &ScriptContext) -> Option<String> {
    ctx.params
        .get("excel_file_path")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `spreadsheet_id` / `sheet_name` из params или config шаблона.
/// При `export_to_excel` ID таблицы не обязателен — цель = путь к `.xlsx`.
pub fn resolve_spreadsheet_target(ctx: &ScriptContext) -> Result<(String, String), ScriptOutput> {
    if export_to_excel(ctx) {
        let path = excel_file_path(ctx).unwrap_or_default();
        let sheet = ctx
            .params
            .get("sheet_name")
            .cloned()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Sheet1".into());
        return Ok((path, sheet));
    }
    let spreadsheet_id = ctx
        .params
        .get("spreadsheet_id")
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| ctx.config.result_spreadsheet_id.clone());
    let sheet_name = ctx
        .params
        .get("sheet_name")
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| ctx.config.result_sheet_name.clone());
    if spreadsheet_id.trim().is_empty() || sheet_name.trim().is_empty() {
        return Err(ScriptOutput {
            rows_written: 0,
            message: crate::core::i18n::t("errors.scripts.templateSheetRequired"),
        });
    }
    Ok((spreadsheet_id, sheet_name))
}

/// Все WB-токены за одно чтение `.env` (ключ — cabinet lower-case).
pub type WbTokenMap = HashMap<String, String>;

pub fn load_wb_tokens() -> Result<WbTokenMap, String> {
    env_keys::load_wb_tokens(&env_keys::default_path())
}

/// Токен WB: param `wb_token` или карта из `.env` (без повторного чтения файла).
pub fn resolve_wb_token_with_map(ctx: &ScriptContext, map: &WbTokenMap, cabinet: &str) -> String {
    let mut token = ctx.params.get("wb_token").cloned().unwrap_or_default();
    if token.trim().is_empty() {
        if let Some(t) = map.get(&cabinet.trim().to_ascii_lowercase()) {
            token = t.clone();
        }
    }
    token
}

/// Все Ozon-пары за одно чтение `.env` (ключ — cabinet lower-case).
pub type OzonPairMap = HashMap<String, (String, String)>;

pub fn load_ozon_pairs() -> Result<OzonPairMap, String> {
    env_keys::load_ozon_pairs(&env_keys::default_path())
}

/// Пара из уже загруженной карты (без повторного чтения `.env`).
pub fn ozon_pair_from_map<'a>(
    map: &'a OzonPairMap,
    cabinet: &str,
) -> Option<&'a (String, String)> {
    map.get(&cabinet.trim().to_ascii_lowercase())
}

/// Заголовки выбранных полей.
pub fn selected_titles(fields: &[ScriptField]) -> Vec<String> {
    fields
        .iter()
        .filter(|f| f.selected)
        .map(|f| f.title.clone())
        .collect()
}

/// Заголовок один раз (selected titles).
pub fn ensure_header_row(
    all_rows: &mut Vec<Vec<String>>,
    header_done: &mut bool,
    fields: &[ScriptField],
) {
    if *header_done {
        return;
    }
    all_rows.push(selected_titles(fields));
    *header_done = true;
}

/// Добавить строки кабинета: заголовок только у первого блока.
pub fn append_cabinet_rows(
    all_rows: &mut Vec<Vec<String>>,
    header_done: &mut bool,
    mut rows: Vec<Vec<String>>,
) {
    if rows.is_empty() {
        return;
    }
    if *header_done {
        rows.remove(0);
    } else {
        *header_done = true;
    }
    all_rows.extend(rows);
}

/// OAuth пользователя (логирует).
pub const CABINET_PARALLELISM: usize = 3;

/// Обход кабинетов чанками: внутри чанка — `try_join_all`, порядок чанков сохраняется.
pub async fn map_cabinets_chunked<T, F, Fut>(
    cabinets: &[String],
    concurrency: usize,
    f: F,
) -> Result<Vec<T>, String>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let n = concurrency.max(1);
    let mut out = Vec::with_capacity(cabinets.len());
    for chunk in cabinets.chunks(n) {
        let futs: Vec<_> = chunk.iter().cloned().map(&f).collect();
        let part = futures_util::future::try_join_all(futs).await?;
        out.extend(part);
    }
    Ok(out)
}

/// Пустой результат «нужен кабинет».
pub fn empty_cabinets_output() -> ScriptOutput {
    ScriptOutput {
        rows_written: 0,
        message: "Нужен кабинет с API-ключом".into(),
    }
}

/// Пустой результат «нет данных».
pub fn empty_data_output() -> ScriptOutput {
    ScriptOutput {
        rows_written: 0,
        message: "Нет данных для записи".into(),
    }
}
