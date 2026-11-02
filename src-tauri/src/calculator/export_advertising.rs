//! Companion-шаблоны Advertising: Инфо + Расходы (паритет EnsureAdvertisingCompanionTemplates).

use crate::app_state::AppState;
use crate::calculator::catalog::TableDef;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::run_log;
use crate::calculator::script_extras;
use crate::core::log::AppLog;
use crate::core::script_field::ScriptField;
use crate::templates::{self, store};
use std::sync::Arc;

pub(crate) const ADVERTISING_INFO_KEY: &str = "Advertising.Info";
pub(crate) const ADVERTISING_EXPENSES_KEY: &str = "Advertising.Expenses";

pub(crate) async fn ensure_and_run_companions(
    state: Arc<AppState>,
    profile: &mut CalculatorProfile,
    spreadsheet_id: &str,
    cabinets: &[String],
    log: &AppLog,
) -> Result<(), String> {
    ensure_companion_templates(state.clone(), profile, spreadsheet_id, cabinets, log).await?;
    run_companions(state, profile, log).await
}

async fn ensure_companion_templates(
    state: Arc<AppState>,
    profile: &mut CalculatorProfile,
    spreadsheet_id: &str,
    cabinets: &[String],
    log: &AppLog,
) -> Result<(), String> {
    let exp_script = state
        .registry
        .get("wb_adv_upd")
        .ok_or_else(|| "Скрипт wb_adv_upd не найден".to_string())?;
    let mut exp = load_or_blank(
        profile,
        ADVERTISING_EXPENSES_KEY,
        "wb_adv_upd",
        "Реклама — Расходы",
        exp_script.default_fields(),
    );
    exp.spreadsheet_id = spreadsheet_id.to_string();
    exp.sheet_name = crate::core::i18n::t("calc.sheets.wb.adExpenses");
    exp.cabinets = cabinets.to_vec();
    exp.marketplace = "wb".into();
    exp.tab_name = "Калькулятор".into();
    script_extras::apply_calculator_script_extras(
        profile,
        &TableDef {
            key: ADVERTISING_EXPENSES_KEY,
            label: "Реклама — Расходы",
            spreadsheet_title: "",
            sheet_name: "Расходы",
            script_key: "wb_adv_upd",
            marketplace: "wb",
        },
        &mut exp,
        &exp_script.default_fields(),
        log,
    );
    let exp_saved = store::upsert(exp)?;
    profile
        .template_ids_by_table_key
        .insert(ADVERTISING_EXPENSES_KEY.to_string(), exp_saved.id);

    let info_script = state
        .registry
        .get("wb_adv_campaigns_stats_new")
        .ok_or_else(|| "Скрипт wb_adv_campaigns_stats_new не найден".to_string())?;
    let mut info = load_or_blank(
        profile,
        ADVERTISING_INFO_KEY,
        "wb_adv_campaigns_stats_new",
        "Реклама — Инфо",
        info_script.default_fields(),
    );
    info.spreadsheet_id = spreadsheet_id.to_string();
    info.sheet_name = crate::core::i18n::t("calc.sheets.wb.adInfo");
    info.second_spreadsheet_id = spreadsheet_id.to_string();
    info.second_sheet_name = "Кампании".into();
    info.cabinets_source_range = "A2:A".into();
    info.id_adv_campaign_source_range = "D2:D".into();
    info.cabinets = cabinets.to_vec();
    info.marketplace = "wb".into();
    info.tab_name = "Калькулятор".into();
    script_extras::apply_calculator_script_extras(
        profile,
        &TableDef {
            key: ADVERTISING_INFO_KEY,
            label: "Реклама — Инфо",
            spreadsheet_title: "",
            sheet_name: "Инфо",
            script_key: "wb_adv_campaigns_stats_new",
            marketplace: "wb",
        },
        &mut info,
        &info_script.default_fields(),
        log,
    );
    let info_saved = store::upsert(info)?;
    profile
        .template_ids_by_table_key
        .insert(ADVERTISING_INFO_KEY.to_string(), info_saved.id);
    Ok(())
}

async fn run_companions(
    state: Arc<AppState>,
    profile: &CalculatorProfile,
    log: &AppLog,
) -> Result<(), String> {
    for key in [ADVERTISING_INFO_KEY, ADVERTISING_EXPENSES_KEY] {
        let Some(id) = profile
            .template_ids_by_table_key
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
        else {
            continue;
        };
        let Some(tpl) = store::get(&id)? else {
            continue;
        };
        let label = if key == ADVERTISING_INFO_KEY {
            "Реклама — Инфо"
        } else {
            "Реклама — Расходы"
        };
        run_log::calc_info(log, format!("[Калькулятор] Выгрузка «{label}»…"));
        templates::run::run_one_template(state.as_ref(), &tpl, false).await?;
        run_log::calc_info(log, format!("[Калькулятор] Готово «{label}»"));
    }
    Ok(())
}

fn load_or_blank(
    profile: &CalculatorProfile,
    table_key: &str,
    script_key: &str,
    label: &str,
    fields: Vec<ScriptField>,
) -> templates::Template {
    if let Some(id) = profile
        .template_ids_by_table_key
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
        .map(|(_, v)| v.clone())
    {
        if let Ok(Some(t)) = store::get(&id) {
            return t;
        }
    }
    let mut t = templates::Template::new_blank(script_key, label, fields, "wb", "Калькулятор");
    t.name = format!("[Калькулятор] {label}");
    t
}
