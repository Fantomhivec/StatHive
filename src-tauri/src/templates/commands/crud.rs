// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::args::{
    MarketplaceArgs, SetSpreadsheetTitleArgs, SetSpreadsheetTitlesArgs, TemplatesGetArgs,
    TemplatesIdArgs, TemplatesListFieldsArgs,
};
use crate::app_state::AppState;
use crate::core::spreadsheet_id;
use crate::templates::access_gate;
use crate::templates::model::normalize_marketplace;
use crate::templates::{self, store};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn templates_snapshot(
    state: State<'_, Arc<AppState>>,
    args: MarketplaceArgs,
) -> Result<serde_json::Value, String> {
    // После restore session пресеты ещё могут быть пусты — без них ложный «нет доступа».
    state.auth.ensure_presets_loaded().await;
    access_gate::require_any_script(&state)?;
    let mut snap = store::snapshot(&args.marketplace)?;
    // Не показывать карточки, которые run сразу deny (excel-only / пресет / memberOk).
    let level = state
        .auth
        .current_user()
        .map(|u| u.access_level)
        .unwrap_or_else(|| "Denied".into());
    let has_oauth = access_gate::session_has_oauth(&state);
    // Участник с Google: один раз снять Excel, залипший от старого force-save.
    if has_oauth && !crate::core::access::is_admin_role(&level) {
        if let Ok(n) = store::clear_stuck_excel_exports_once() {
            if n > 0 {
                snap = store::snapshot(&args.marketplace)?;
            }
        }
    }
    let market = normalize_marketplace(&args.marketplace);
    let allowed = |script_key: &str| {
        let member_ok = state.registry.is_member_ok(script_key);
        crate::core::access::can_run_script(&level, script_key, has_oauth, member_ok)
            && crate::scripts::groups::script_belongs_to_marketplace(script_key, &market)
    };
    snap.templates.retain(|t| allowed(&t.script_key));
    {
        let allowed_ids: std::collections::HashSet<&str> =
            snap.templates.iter().map(|t| t.id.as_str()).collect();
        for g in &mut snap.groups {
            g.template_ids
                .retain(|id| allowed_ids.contains(id.as_str()));
        }
    }
    snap.groups.retain(|g| !g.template_ids.is_empty());
    let in_group: std::collections::HashSet<String> = snap
        .groups
        .iter()
        .flat_map(|g| g.template_ids.iter().cloned())
        .collect();
    snap.standalone = snap
        .templates
        .iter()
        .filter(|t| !in_group.contains(&t.id))
        .cloned()
        .collect();
    Ok(serde_json::to_value(snap).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn templates_get(
    state: State<'_, Arc<AppState>>,
    args: TemplatesGetArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_session(&state)?;
    if args.id.trim().is_empty() {
        let market = normalize_marketplace(&args.marketplace);
        let tab = if args.tab_name.trim().is_empty() {
            store::selected_tab_for(&market)?
        } else {
            args.tab_name.trim().to_string()
        };
        let level = state
            .auth
            .current_user()
            .map(|u| u.access_level)
            .unwrap_or_else(|| "Denied".into());
        let has_oauth = access_gate::session_has_oauth(&state);
        let requested = args.script_key.trim().to_string();
        let key = if requested.is_empty()
            || !crate::scripts::groups::script_belongs_to_marketplace(&requested, &market)
        {
            crate::scripts::groups::default_script_key(&state.registry, &market, &|k, member_ok| {
                crate::core::access::can_run_script(&level, k, has_oauth, member_ok)
            })
        } else {
            requested
        };
        if !crate::scripts::groups::script_belongs_to_marketplace(&key, &market) {
            return Err(format!(
                "Скрипт «{key}» не относится к площадке «{market}»"
            ));
        }
        access_gate::require_script(&state, &key)?;
        let mut tpl = templates::new_template(&state.registry, &key, &market, &tab)?;
        if access_gate::templates_excel_only(&state) {
            tpl.export_to_excel = true;
        }
        return Ok(serde_json::json!({ "template": tpl }));
    }
    let mut tpl = store::get(args.id.trim())?.ok_or_else(|| "Шаблон не найден".to_string())?;
    access_gate::require_script(&state, &tpl.script_key)?;
    tpl = templates::merge_fields_on_get(&state.registry, tpl)?;
    if access_gate::templates_excel_only(&state) {
        tpl.export_to_excel = true;
    }
    Ok(serde_json::json!({ "template": tpl }))
}

/// Кэш названия Google-документа на карточке (без полной перезаписи шаблона).
#[tauri::command]
pub fn templates_set_spreadsheet_title(
    state: State<'_, Arc<AppState>>,
    args: SetSpreadsheetTitleArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let ok = store::backfill_spreadsheet_target(
        args.id.trim(),
        "",
        "",
        args.spreadsheet_title.trim(),
    )?;
    Ok(serde_json::json!({ "ok": ok }))
}

/// Пачка названий документов — один load/save store.
#[tauri::command]
pub fn templates_set_spreadsheet_titles(
    state: State<'_, Arc<AppState>>,
    args: SetSpreadsheetTitlesArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let owned: Vec<(String, String)> = args
        .items
        .into_iter()
        .map(|i| (i.id, i.spreadsheet_title))
        .collect();
    let refs: Vec<(&str, &str, &str, &str)> = owned
        .iter()
        .map(|(id, title)| (id.as_str(), "", "", title.as_str()))
        .collect();
    let ok = store::backfill_spreadsheet_targets(&refs)?;
    Ok(serde_json::json!({ "ok": ok }))
}

#[tauri::command]
pub fn templates_save(
    state: State<'_, Arc<AppState>>,
    template: crate::templates::Template,
) -> Result<serde_json::Value, String> {
    if template.name.trim().is_empty() {
        return Err("Укажите имя шаблона".into());
    }
    let market = normalize_marketplace(&template.marketplace);
    if state.registry.get(&template.script_key).is_none() {
        return Err(format!("Скрипт не найден: {}", template.script_key));
    }
    if !crate::scripts::groups::script_belongs_to_marketplace(&template.script_key, &market) {
        return Err(format!(
            "Скрипт «{}» нельзя сохранить в шаблонах «{}»",
            template.script_key, market
        ));
    }
    access_gate::require_script(&state, &template.script_key)?;
    let mut tpl = template;
    // Excel-only — только runtime (run/get). В store не пишем true: иначе после
    // подключения Google все шаблоны навсегда остаются в Excel.
    let excel_locked = access_gate::templates_excel_only(&state);
    if excel_locked {
        tpl.export_to_excel = false;
    }
    access_gate::reject_google_write_if_locked(&state, excel_locked || tpl.export_to_excel)?;
    let (id, _) = spreadsheet_id::normalize(&tpl.spreadsheet_id);
    tpl.spreadsheet_id = id;
    tpl.marketplace = normalize_marketplace(&tpl.marketplace);
    let saved = store::upsert(tpl)?;
    state.log.info(format!("Шаблон сохранён: {}", saved.name));
    Ok(serde_json::json!({ "template": saved }))
}

#[tauri::command]
pub fn templates_delete(
    state: State<'_, Arc<AppState>>,
    args: TemplatesIdArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let ok = store::delete(args.id.trim())?;
    if ok {
        state.log.info("Шаблон удалён");
    }
    Ok(serde_json::json!({ "ok": ok }))
}

#[tauri::command]
pub fn templates_list_fields(
    state: State<'_, Arc<AppState>>,
    args: TemplatesListFieldsArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_script(&state, args.script_key.trim())?;
    let fields = templates::list_fields_for_script(
        &state.registry,
        args.script_key.trim(),
        if args.selected_fields.is_empty() {
            None
        } else {
            Some(&args.selected_fields)
        },
    )?;
    Ok(serde_json::json!({ "fields": fields }))
}
