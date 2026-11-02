//! Общие helpers запуска скриптов (кабинеты, поля, Sheets, токены).

use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::script::ScriptContext;
use crate::core::script_field::ScriptField;

/// Числовой nmId/SKU == 0 (`0`, `0.0`, `0,0`…). Нечисловые ключи — false.
/// Паритет `CalculatorArticleIndex.IsZeroNumericId`.

use super::resolve::{
    dry_run, excel_file_path, export_to_excel, is_zero_numeric_id, resolve_fields,
};

pub async fn google_sheets_client(ctx: &ScriptContext) -> Result<GoogleSheetsClient, String> {
    ctx.log.info("Google OAuth пользователя…");
    GoogleSheetsClient::from_user_oauth()
        .await
        .map_err(|e| format!("Google OAuth: {e}"))
}

/// clear_and_write, Excel `.xlsx` или инкрементальная запись (если sync в params).
pub async fn write_sheet_rows_with(
    ctx: &ScriptContext,
    client: &GoogleSheetsClient,
    spreadsheet_id: &str,
    sheet_name: &str,
    rows: Vec<Vec<String>>,
) -> Result<usize, String> {
    ctx.check_cancelled().map_err(|e| e.to_string())?;
    if rows.is_empty() {
        return Ok(0);
    }
    let (rows, skipped_zero) = filter_rows_with_zero_article_ids(rows);
    if skipped_zero > 0 {
        ctx.log.info(format!(
            "Пропущено строк с nmId/SKU = 0: {skipped_zero}."
        ));
    }
    if rows.is_empty() {
        return Ok(0);
    }

    let rows = apply_write_sort(ctx, rows);

    if dry_run(ctx) {
        let n = rows.len();
        ctx.log.info(format!(
            "Пробный прогон: запись пропущена ({n} строк, Sheets/Excel не меняются)."
        ));
        return Ok(n);
    }

    if export_to_excel(ctx) {
        let path = excel_file_path(ctx)
            .filter(|p| !p.is_empty())
            .or_else(|| {
                let sid = spreadsheet_id.trim();
                if sid.is_empty() {
                    None
                } else {
                    Some(sid.to_string())
                }
            })
            .ok_or_else(|| crate::core::i18n::t("errors.scripts.excelPathRequired"))?;
        let sheet = if sheet_name.trim().is_empty() {
            "Sheet1".to_string()
        } else {
            sheet_name.trim().to_string()
        };
        let written_total = rows.len();
        let (headers, data_rows) = if rows.len() <= 1 {
            (rows.first().cloned().unwrap_or_default(), Vec::new())
        } else {
            let mut it = rows.into_iter();
            let headers = it.next().unwrap_or_default();
            (headers, it.collect())
        };
        ctx.log.info(format!("Запись в Excel: {written_total} строк → {path}"));
        crate::core::excel::write_simple_xlsx(&crate::core::excel::SimpleSheetWrite {
            path: path.clone(),
            sheet_name: sheet,
            headers,
            rows: data_rows,
        })?;
        ctx.log.info(format!("Записано строк в Excel: {written_total}"));
        return Ok(written_total);
    }

    if let Some(keys) = resolve_sync_key_indexes(ctx, &rows) {
        ctx.log.info(format!(
            "Запись в Google Sheets: {} строк, инкрементальная синхронизация…",
            rows.len()
        ));
        let allow_delete = !sync_do_not_delete(ctx);
        let stats = client
            .write_incremental(
                spreadsheet_id.trim(),
                sheet_name.trim(),
                rows,
                &keys,
                allow_delete,
            )
            .await
            .map_err(|e| format!("Google Sheets: {e}"))?;
        ctx.log.info(format!(
            "Инкрементальная запись: изменено {}, добавлено {}, удалено {}",
            stats.updated_rows, stats.inserted_rows, stats.deleted_rows
        ));
        return Ok(stats.updated_rows + stats.inserted_rows);
    }

    let clear_only = ctx
        .params
        .get("clear_only_written_range")
        .map(|s| {
            let t = s.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes"
        })
        .unwrap_or(false);
    let clear_mode = if clear_only {
        "только записанный диапазон"
    } else {
        "весь лист"
    };
    ctx.log.info(format!(
        "Запись в Google Sheets: {} строк, очистка = {clear_mode}…",
        rows.len()
    ));
    let written = client
        .clear_and_write(
            spreadsheet_id.trim(),
            sheet_name.trim(),
            rows,
            clear_only,
        )
        .await
        .map_err(|e| format!("Google Sheets: {e}"))?;
    ctx.log.info(format!("Записано строк: {written}"));
    Ok(written)
}

/// `TryApplyCentralSort`: сортировка data-строк по выбранному полю (заголовок не трогаем).
fn apply_write_sort(ctx: &ScriptContext, mut rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let field = ctx
        .params
        .get("write_sort_field_name")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let Some(field) = field else {
        return rows;
    };
    if rows.len() <= 1 {
        return rows;
    }

    let fields = resolve_fields(ctx, Vec::new());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let header = &rows[0];
    let col = selected
        .iter()
        .position(|f| f.name.eq_ignore_ascii_case(field))
        .or_else(|| {
            header.iter().position(|h| {
                h.eq_ignore_ascii_case(field)
                    || selected
                        .iter()
                        .find(|f| f.name.eq_ignore_ascii_case(field))
                        .is_some_and(|f| h.eq_ignore_ascii_case(&f.title))
            })
        });
    let Some(col) = col else {
        ctx.log.info(format!(
            "Сортировка записи: поле «{field}» не найдено — пропуск."
        ));
        return rows;
    };

    let desc = ctx
        .params
        .get("write_sort_direction")
        .map(|s| s.trim().eq_ignore_ascii_case("desc"))
        .unwrap_or(false);
    let dir_label = if desc { "desc" } else { "asc" };
    ctx.log.info(format!(
        "Настройки записи: сортировка = {field} ({dir_label})."
    ));

    let header_row = rows.remove(0);
    rows.sort_by(|a, b| {
        let cmp = compare_sort_cells(a.get(col).map(String::as_str).unwrap_or(""), b.get(col).map(String::as_str).unwrap_or(""));
        if desc {
            cmp.reverse()
        } else {
            cmp
        }
    });
    rows.insert(0, header_row);
    rows
}

fn compare_sort_cells(a: &str, b: &str) -> std::cmp::Ordering {
    let a = a.trim();
    let b = b.trim();
    let an = parse_sort_number(a);
    let bn = parse_sort_number(b);
    match (an, bn) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.cmp(b),
    }
}


fn parse_sort_number(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let n = s.replace('\u{00a0}', "").replace(' ', "").replace(',', ".");
    n.parse::<f64>().ok()
}

fn sync_do_not_delete(ctx: &ScriptContext) -> bool {
    ctx.params
        .get("sync_settings_json")
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|v| {
            v.get("do_not_delete_old_rows")
                .or_else(|| v.get("DoNotDeleteOldRows"))
                .and_then(|x| x.as_bool())
        })
        .unwrap_or(false)
}

/// Индексы ключей по именам полей из sync_settings + selected_fields_json / заголовок.
fn resolve_sync_key_indexes(ctx: &ScriptContext, rows: &[Vec<String>]) -> Option<Vec<usize>> {
    let raw = ctx.params.get("sync_settings_json")?;
    let sync: serde_json::Value = serde_json::from_str(raw).ok()?;
    let enabled = sync
        .get("enable_change_sync")
        .or_else(|| sync.get("EnableChangeSync"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    let k1 = sync
        .get("key_field_1")
        .or_else(|| sync.get("KeyField1"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    let k2 = sync
        .get("key_field_2")
        .or_else(|| sync.get("KeyField2"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    if k1.is_empty() && k2.is_empty() {
        ctx.log
            .info("Инкрементальная запись отключена: ключевые поля недоступны, полная перезапись.");
        return None;
    }
    let fields = resolve_fields(ctx, Vec::new());
    let selected: Vec<&ScriptField> = fields.iter().filter(|f| f.selected).collect();
    let header = rows.first()?;
    let mut indexes = Vec::new();
    for key_name in [k1, k2] {
        if key_name.is_empty() {
            continue;
        }
        let by_field = selected
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(key_name));
        let by_header = header.iter().position(|h| {
            h.eq_ignore_ascii_case(key_name)
                || selected
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(key_name))
                    .is_some_and(|f| h.eq_ignore_ascii_case(&f.title))
        });
        match by_field.or(by_header) {
            Some(idx) => indexes.push(idx),
            None => {
                ctx.log.info(format!(
                    "Инкрементальная запись: ключ «{key_name}» не найден, полная перезапись."
                ));
                return None;
            }
        }
    }
    if indexes.is_empty() {
        None
    } else {
        Some(indexes)
    }
}

/// OAuth + clear_and_write; логирует прогресс.
/// Строки с nmId/SKU = 0 не пишем (паритет `WriteRowsAsync` / `RemoveRowsWithZeroArticleIds`).
pub async fn write_sheet_rows(
    ctx: &ScriptContext,
    spreadsheet_id: &str,
    sheet_name: &str,
    rows: Vec<Vec<String>>,
) -> Result<usize, String> {
    let client = google_sheets_client(ctx).await?;
    write_sheet_rows_with(ctx, &client, spreadsheet_id, sheet_name, rows).await
}

fn looks_like_article_id_column(name_or_title: &str) -> bool {
    let s = name_or_title.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    if lower == "sku"
        || lower == "nmid"
        || lower == "nm_id"
        || lower.contains("nmid")
    {
        return true;
    }
    if s.eq_ignore_ascii_case("SKU")
        || lower.starts_with("sku ")
        || lower.starts_with("sku (")
    {
        return true;
    }
    if s.starts_with("Артикул WB")
        || s.starts_with("Артикул ВБ")
        || s.starts_with("Артикул")
    {
        return true;
    }
    false
}

fn filter_rows_with_zero_article_ids(rows: Vec<Vec<String>>) -> (Vec<Vec<String>>, usize) {
    if rows.len() <= 1 {
        return (rows, 0);
    }
    let header = &rows[0];
    let id_cols: Vec<usize> = header
        .iter()
        .enumerate()
        .filter(|(_, h)| looks_like_article_id_column(h))
        .map(|(i, _)| i)
        .collect();
    if id_cols.is_empty() {
        return (rows, 0);
    }
    let mut kept = Vec::with_capacity(rows.len());
    kept.push(rows[0].clone());
    let mut removed = 0usize;
    for row in rows.into_iter().skip(1) {
        let drop = id_cols.iter().any(|&col| {
            row.get(col)
                .map(|c| is_zero_numeric_id(c))
                .unwrap_or(false)
        });
        if drop {
            removed += 1;
        } else {
            kept.push(row);
        }
    }
    (kept, removed)
}

