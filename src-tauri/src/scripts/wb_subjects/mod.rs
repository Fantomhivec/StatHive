//! `subjects_list` — предметы WB → Sheets.

use crate::core::rate_profiles::subjects_list as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::templates::SubjectsListFilters;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

pub struct SubjectsScript;

impl Script for SubjectsScript {
    fn key(&self) -> &'static str {
        "subjects_list"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.subjects_list")
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
    ctx.log.info("Скрипт «Список категорий»: старт");
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
    let filters = parse_filters(ctx);
    let wb_tokens = common::load_wb_tokens()?;
    let parts = common::map_cabinets_chunked(
        &cabinets,
        common::CABINET_PARALLELISM,
        |cabinet| {
            let token = common::resolve_wb_token_with_map(ctx, &wb_tokens, &cabinet);
            let filters = filters.clone();
            async move {
                if token.trim().is_empty() {
                    ctx.log
                        .warn(format!("Токен WB не задан для кабинета «{cabinet}»"));
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let subjects = fetch_paged(ctx, &cabinet, token.trim(), &filters).await?;
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: предметов {}", subjects.len()));
                Ok((cabinet, subjects))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, subjects) in parts {
        if subjects.is_empty() {
            continue;
        }
        for subj in &subjects {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(if f.name == "cabinet" {
                    cabinet.clone()
                } else {
                    field_val(subj, &f.name)
                });
            }
            all_rows.push(row);
        }
        total += subjects.len();
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} предметов на лист «{sheet_name}»"),
    })
}

fn parse_filters(ctx: &ScriptContext) -> SubjectsListFilters {
    ctx.params
        .get("subjects_filters_json")
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default()
}

async fn fetch_paged(
    ctx: &ScriptContext,
    cabinet: &str,
    token: &str,
    filters: &SubjectsListFilters,
) -> Result<Vec<Value>, String> {
    let mut all = Vec::new();
    let mut offset = filters.offset.unwrap_or(0).max(0) as i64;
    let profile = rate::profile();
    let locale = if filters.locale.trim().is_empty() {
        "ru"
    } else {
        filters.locale.trim()
    };
    let page_limit = 1000i64;

    loop {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let mut q = format!(
            "https://content-api.wildberries.ru/content/v2/object/all?locale={}&limit={page_limit}&offset={offset}",
            urlencoding_simple(locale)
        );
        if !filters.name.trim().is_empty() {
            q.push_str(&format!("&name={}", urlencoding_simple(filters.name.trim())));
        }
        if let Some(pid) = filters.parent_id {
            q.push_str(&format!("&parentID={pid}"));
        }
        let v = ctx
            .request
            .get_json(
                Limit {
                    cabinet,
                    scope: rate::SCOPE,
                    profile: &profile,
                },
                &q,
                Auth::Bearer(token),
            )
            .await?;
        let page = v
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        let n = page.len();
        all.extend(page);
        if n == 0 || n < page_limit as usize {
            break;
        }
        offset += page_limit;
    }
    Ok(all)
}

fn field_val(subj: &Value, name: &str) -> String {
    let keys: &[&str] = match name {
        "subjectID" => &["subjectID", "subjectId"],
        "parentID" => &["parentID", "parentId"],
        "subjectName" => &["subjectName"],
        "parentName" => &["parentName"],
        "error" => &["error"],
        "errorText" => &["errorText"],
        "additionalErrors" => &["additionalErrors"],
        _ => &[name],
    };
    for k in keys {
        if let Some(x) = subj.get(*k) {
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

fn urlencoding_simple(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.subjects_list.cabinet"), true),
        ScriptField::new("subjectID", crate::core::i18n::t("fields.subjects_list.subjectID"), true),
        ScriptField::new("parentID", crate::core::i18n::t("fields.subjects_list.parentID"), true),
        ScriptField::new("subjectName", crate::core::i18n::t("fields.subjects_list.subjectName"), true),
        ScriptField::new("parentName", crate::core::i18n::t("fields.subjects_list.parentName"), true),
        ScriptField::new("error", crate::core::i18n::t("fields.subjects_list.error"), false),
        ScriptField::new("errorText", crate::core::i18n::t("fields.subjects_list.errorText"), false),
        ScriptField::new("additionalErrors", crate::core::i18n::t("fields.subjects_list.additionalErrors"), false),
    ]
}
