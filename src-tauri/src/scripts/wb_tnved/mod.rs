//! `wb_tnved_codes` — ТН ВЭД по предметам → Sheets.

pub mod fields;

use crate::core::rate_profiles::wb_tnved_codes as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::{common, sheet_ranges};
use self::fields::default_fields;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct WbTnvedScript;

impl Script for WbTnvedScript {
    fn key(&self) -> &'static str {
        "wb_tnved_codes"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_tnved_codes")
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
    ctx.log.info("Скрипт «ТНВЭД-код»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<_> = fields
        .iter()
        .filter(|f| f.selected && f.name != "cabinet")
        .cloned()
        .collect();
    if selected.is_empty() {
        return Err("Не выбраны поля для выгрузки".into());
    }

    let wb_tokens = common::load_wb_tokens()?;
    let (cabinet, token) = pick_token(ctx, &wb_tokens)?;
    let custom = ctx
        .params
        .get("use_custom_subject_ids")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let client = common::google_sheets_client(ctx).await?;
    let subjects = if custom {
        load_custom_subjects(ctx, &client).await?
    } else {
        fetch_all_subjects(ctx, &cabinet, &token).await?
    };

    let mut all_rows: Vec<Vec<String>> = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    let profile = rate::profile();
    let n_subj = subjects.len();

    for (i, (sid, sname)) in subjects.iter().enumerate() {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = format!(
            "https://content-api.wildberries.ru/content/v2/directory/tnved?subjectID={sid}&locale=ru"
        );
        let v = match ctx
            .request
            .get_json(
                Limit {
                    cabinet: &cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                &url,
                Auth::Bearer(&token),
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                ctx.log
                    .warn(format!("subjectID={sid}: ошибка TNVED: {e}"));
                continue;
            }
        };
        let items = v
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(match f.name.as_str() {
                    "subjectID" => sid.to_string(),
                    "subjectName" => sname.clone(),
                    "tnved" => jstr(item, &["tnved", "tnVed"]),
                    "isKiz" => jstr(item, &["isKiz", "isKIZ"]),
                    _ => String::new(),
                });
            }
            all_rows.push(row);
            total += 1;
        }
        if (i + 1) % 50 == 0 || i + 1 == n_subj {
            let pct = if n_subj == 0 {
                100
            } else {
                ((i + 1) * 100) / n_subj
            };
            ctx.log
                .info(format!("Запрос {} из {n_subj} ({pct}%)", i + 1));
        }
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} кодов ТН ВЭД на лист «{sheet_name}»"),
    })
}

pub(crate) fn pick_token(
    ctx: &ScriptContext,
    map: &common::WbTokenMap,
) -> Result<(String, String), String> {
    for cab in common::resolve_cabinets(ctx) {
        let t = common::resolve_wb_token_with_map(ctx, map, &cab);
        if !t.trim().is_empty() {
            return Ok((cab, t));
        }
    }
    if let Some((k, v)) = map.iter().next() {
        if !v.trim().is_empty() {
            return Ok((k.clone(), v.clone()));
        }
    }
    Err("Не найден валидный WB API ключ".into())
}

pub(crate) async fn fetch_all_subjects(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
) -> Result<Vec<(i64, String)>, String> {
    use crate::core::rate_profiles::subjects_list as subj_rate;
    let profile = subj_rate::profile();
    let mut out = Vec::new();
    let mut offset = 0i64;
    let page_limit = 1000i64;
    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = format!(
            "https://content-api.wildberries.ru/content/v2/object/all?locale=ru&limit={page_limit}&offset={offset}"
        );
        let v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: subj_rate::SCOPE,
                    profile: &profile,
                },
                &url,
                Auth::Bearer(token),
            )
            .await?;
        let page = v
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        let n = page.len();
        for s in page {
            let id = s
                .get("subjectID")
                .or_else(|| s.get("subjectId"))
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            if id <= 0 {
                continue;
            }
            let name = s
                .get("subjectName")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            out.push((id, name));
        }
        if n == 0 || n < page_limit as usize {
            break;
        }
        offset += page_limit;
    }
    out.sort_by_key(|(id, _)| *id);
    out.dedup_by_key(|(id, _)| *id);
    Ok(out)
}

pub(crate) async fn load_custom_subjects(
    ctx: &ScriptContext,
    client: &crate::core::google_sheets::GoogleSheetsClient,
) -> Result<Vec<(i64, String)>, String> {
    let sid = ctx
        .params
        .get("second_spreadsheet_id")
        .or_else(|| ctx.params.get("source_spreadsheet_id"))
        .cloned()
        .unwrap_or_default();
    let sheet = ctx
        .params
        .get("second_sheet_name")
        .or_else(|| ctx.params.get("source_sheet_name"))
        .cloned()
        .unwrap_or_default();
    let id_range = ctx
        .params
        .get("subject_id_source_range")
        .cloned()
        .unwrap_or_default();
    if sid.trim().is_empty() || sheet.trim().is_empty() || id_range.trim().is_empty() {
        return Err(
            "При режиме пользовательских ID укажите вторую таблицу, лист и диапазон ID категорий"
                .into(),
        );
    }
    let name_range = ctx
        .params
        .get("subject_name_source_range")
        .cloned()
        .unwrap_or_default();
    let ids = sheet_ranges::read_first_cols(
        client,
        sid.trim(),
        sheet.trim(),
        &[id_range.trim()],
    )
    .await?
    .into_iter()
    .next()
    .unwrap_or_default();
    let names = if name_range.trim().is_empty() {
        Vec::new()
    } else {
        sheet_ranges::read_first_cols(
            client,
            sid.trim(),
            sheet.trim(),
            &[name_range.trim()],
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default()
    };
    let mut map: HashMap<i64, String> = HashMap::new();
    for (i, raw) in ids.iter().enumerate() {
        let Some(id) = sheet_ranges::parse_i64_cell(raw).filter(|n| *n > 0) else {
            continue;
        };
        let name = names.get(i).cloned().unwrap_or_default();
        map.entry(id).or_insert(name);
    }
    let mut out: Vec<_> = map.into_iter().collect();
    out.sort_by_key(|(id, _)| *id);
    Ok(out)
}

fn jstr(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(x) = v.get(*k) {
            return match x {
                Value::String(s) => s.clone(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
        }
    }
    String::new()
}
