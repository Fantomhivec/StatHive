//! `categories_parent` — родительские категории WB → Sheets.

use crate::core::rate_profiles::categories_parent as rate;
use crate::core::request_sender::{Auth, Limit};
use crate::core::script::{Script, ScriptContext, ScriptOutput, ScriptResult};
use crate::core::script_field::ScriptField;
use crate::scripts::common;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

const URL: &str = "https://content-api.wildberries.ru/content/v2/object/parent/all?locale=ru";

pub struct CategoriesScript;

impl Script for CategoriesScript {
    fn key(&self) -> &'static str {
        "categories_parent"
    }

    fn title(&self) -> String {
        crate::core::i18n::t("scripts.titles.categories_parent")
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
    ctx.log.info("Скрипт «ID категорий»: старт");
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
                    return Ok((cabinet, Vec::<Value>::new()));
                }
                let profile = rate::profile();
                let v = ctx
                    .request
                    .get_json(
                        Limit {
                            cabinet: &cabinet,
                            scope: rate::SCOPE,
                            profile: &profile,
                        },
                        URL,
                        Auth::Bearer(token.trim()),
                    )
                    .await?;
                let cats = extract_data_array(&v);
                ctx.log
                    .info(format!("Кабинет «{cabinet}»: категорий {}", cats.len()));
                Ok((cabinet, cats))
            }
        },
    )
    .await?;

    let mut all_rows: Vec<Vec<String>> = vec![selected.iter().map(|f| f.title.clone()).collect()];
    let mut total = 0usize;
    for (cabinet, cats) in parts {
        if cats.is_empty() {
            continue;
        }
        for cat in &cats {
            let mut row = Vec::with_capacity(selected.len());
            for f in &selected {
                row.push(if f.name == "cabinet" {
                    cabinet.clone()
                } else {
                    field_val(cat, &f.name)
                });
            }
            all_rows.push(row);
        }
        total += cats.len();
    }

    if all_rows.len() <= 1 {
        return Ok(common::empty_data_output());
    }
    common::write_sheet_rows(ctx, &spreadsheet_id, &sheet_name, all_rows).await?;
    Ok(ScriptOutput {
        rows_written: total,
        message: format!("Записано {total} категорий на лист «{sheet_name}»"),
    })
}

fn extract_data_array(v: &Value) -> Vec<Value> {
    v.get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
}

fn field_val(cat: &Value, name: &str) -> String {
    match cat.get(name) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

// --- fields (merged) ---
pub fn default_fields() -> Vec<ScriptField> {
    vec![
        ScriptField::new("cabinet", crate::core::i18n::t("fields.categories_parent.cabinet"), true),
        ScriptField::new("name", crate::core::i18n::t("fields.categories_parent.name"), true),
        ScriptField::new("id", crate::core::i18n::t("fields.categories_parent.id"), true),
        ScriptField::new("isVisible", crate::core::i18n::t("fields.categories_parent.isVisible"), true),
        ScriptField::new("error", crate::core::i18n::t("fields.categories_parent.error"), false),
        ScriptField::new("errorText", crate::core::i18n::t("fields.categories_parent.errorText"), false),
        ScriptField::new("additionalErrors", crate::core::i18n::t("fields.categories_parent.additionalErrors"), false),
    ]
}
