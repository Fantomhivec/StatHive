//! `wb_object_charcs` — GET content `/content/v2/object/charcs/{subjectId}`.

use crate::core::rate_profiles::wb_object_charcs as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use crate::scripts::wb_tnved::{fetch_all_subjects, load_custom_subjects, pick_token};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

pub struct WbObjectCharcsScript;

impl Script for WbObjectCharcsScript {
    fn key(&self) -> &'static str {
        "wb_object_charcs"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.wb_object_charcs")
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
    ctx.log.info("Скрипт «Характеристики предмета»: старт");
    let (spreadsheet_id, sheet_name) = match common::resolve_spreadsheet_target(ctx) {
        Ok(t) => t,
        Err(o) => return Ok(o),
    };
    let fields = common::resolve_fields(ctx, default_fields());
    let selected: Vec<_> = fields.iter().filter(|f| f.selected).cloned().collect();
    if selected.is_empty() {
        return Err("Не выбраны поля для выгрузки".into());
    }
    let wb_tokens = common::load_wb_tokens()?;
    let (cabinet, token) = pick_token(ctx, &wb_tokens)?;
    let custom = ctx
        .params
        .get("use_custom_subject_ids")
        .is_some_and(|s| s.eq_ignore_ascii_case("true"));
    let client = common::google_sheets_client(ctx).await?;
    let subjects = if custom {
        load_custom_subjects(ctx, &client).await?
    } else {
        fetch_all_subjects(ctx, &cabinet, &token).await?
    };
    if subjects.is_empty() {
        return Ok(common::empty_data_output());
    }

    let mut all_rows = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    let profile = rate::profile();
    let n_subj = subjects.len();
    for (i, (sid, fallback_name)) in subjects.iter().enumerate() {
        ctx.check_cancelled().map_err(|e| e.to_string())?;
        let url = format!(
            "https://content-api.wildberries.ru/content/v2/object/charcs/{sid}"
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
                ctx.log.warn(format!("subjectId={sid}: {e}"));
                continue;
            }
        };
        if v.get("error").and_then(|e| e.as_bool()).unwrap_or(false) {
            let text = v
                .get("errorText")
                .and_then(|t| t.as_str())
                .unwrap_or("error=true");
            ctx.log.warn(format!("subjectId={sid}: {text}"));
            continue;
        }
        let items = v
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let row = selected
                .iter()
                .map(|f| cell(&f.name, *sid, fallback_name, item))
                .collect();
            all_rows.push(row);
            total += 1;
        }
        if (i + 1) % 50 == 0 || i + 1 == n_subj {
            let pct = ((i + 1) * 100) / n_subj;
            ctx.log
                .info(format!("Запрос {} из {n_subj} ({pct}%)", i + 1));
        }
    }
    if total == 0 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows_with(ctx, &client, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано характеристик: {total} → «{sheet_name}»"),
    })
}

fn cell(name: &str, sid: i64, fallback_name: &str, item: &Value) -> String {
    match name {
        "subjectId" => {
            let api = scalar(item.get("subjectID").or_else(|| item.get("subjectId")));
            if api.is_empty() {
                sid.to_string()
            } else {
                api
            }
        }
        "subjectName" => {
            let api = scalar(item.get("subjectName"));
            if api.is_empty() {
                fallback_name.to_string()
            } else {
                api
            }
        }
        "charcID" => scalar(item.get("charcID").or_else(|| item.get("charcId"))),
        "name" => scalar(item.get("name")),
        "required" => bool_cell(item.get("required")),
        "unitName" => scalar(item.get("unitName")),
        "maxCount" => scalar(item.get("maxCount")),
        "popular" => bool_cell(item.get("popular")),
        "charcType" => map_charc_type(item.get("charcType").and_then(|v| v.as_i64()).unwrap_or(-1)),
        _ => String::new(),
    }
}

fn map_charc_type(n: i64) -> String {
    match n {
        1 => "массив строк".into(),
        4 => "число (целое или с десятичной дробью)".into(),
        0 => "характеристика не используется".into(),
        other => format!("неизвестный тип ({other})"),
    }
}

fn bool_cell(v: Option<&Value>) -> String {
    match v.and_then(|x| x.as_bool()) {
        Some(true) => "true".into(),
        Some(false) => "false".into(),
        None => String::new(),
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
        "subjectId",
        "subjectName",
        "charcID",
        "name",
        "required",
        "unitName",
        "maxCount",
        "popular",
        "charcType",
    ]
    .into_iter()
    .map(|n| ScriptField {
        name: n.into(),
        title: crate::core::i18n::t(&format!("fields.wb_object_charcs.{n}")),
        selected: true,
    })
    .collect()
}
