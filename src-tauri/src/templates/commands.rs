use crate::app_state::AppState;
use crate::core::spreadsheet_id;
use crate::templates::access_gate;
use crate::templates::model::{normalize_marketplace, TemplateGroup};
use crate::templates::{self, run, store};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Deserialize)]
pub struct MarketplaceArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
}

fn default_wb() -> String {
    "wb".into()
}

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
    let allowed = |script_key: &str| {
        let member_ok = state.registry.is_member_ok(script_key);
        crate::core::access::can_run_script(&level, script_key, has_oauth, member_ok)
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

#[derive(Deserialize)]
pub struct TemplatesGetArgs {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub script_key: String,
    #[serde(default = "default_wb")]
    pub marketplace: String,
    #[serde(default)]
    pub tab_name: String,
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
        let key = if args.script_key.trim().is_empty() {
            state
                .registry
                .list_with_meta()
                .into_iter()
                .find(|(k, _, member_ok)| {
                    crate::core::access::can_run_script(&level, k, has_oauth, *member_ok)
                })
                .map(|(k, _, _)| k)
                .unwrap_or_else(|| {
                    if market == "ozon" {
                        "ozon_pending".into()
                    } else {
                        "cards_list".into()
                    }
                })
        } else {
            args.script_key.trim().to_string()
        };
        access_gate::require_script(&state, &key)?;
        let mut tpl = if state.registry.get(&key).is_some() {
            templates::new_template(&state.registry, &key, &market, &tab)?
        } else {
            crate::templates::Template::new_blank(
                &key,
                if market == "ozon" {
                    "OZON (скоро)"
                } else {
                    &key
                },
                Vec::new(),
                &market,
                &tab,
            )
        };
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

#[derive(Deserialize)]
pub struct SetSpreadsheetTitleArgs {
    pub id: String,
    #[serde(default)]
    pub spreadsheet_title: String,
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

#[derive(Deserialize)]
pub struct SetSpreadsheetTitlesArgs {
    #[serde(default)]
    pub items: Vec<SetSpreadsheetTitleArgs>,
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
    if market == "wb" && state.registry.get(&template.script_key).is_none() {
        return Err(format!("Скрипт не найден: {}", template.script_key));
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

#[derive(Deserialize)]
pub struct TemplatesIdArgs {
    pub id: String,
    /// Пробный прогон: скрипт выполняется, запись в Sheets/Excel пропускается.
    #[serde(default)]
    pub dry_run: bool,
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

#[derive(Deserialize)]
pub struct TemplatesListFieldsArgs {
    pub script_key: String,
    #[serde(default)]
    pub selected_fields: Vec<crate::core::script_field::ScriptField>,
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

#[tauri::command]
pub async fn templates_run(
    state: State<'_, Arc<AppState>>,
    args: TemplatesIdArgs,
) -> Result<serde_json::Value, String> {
    let tpl = store::get(args.id.trim())?.ok_or_else(|| "Шаблон не найден".to_string())?;
    let (rows, message) = run::run_one_template(&state, &tpl, args.dry_run).await?;
    Ok(serde_json::json!({
        "ok": true,
        "rows_written": rows,
        "message": message,
        "dry_run": args.dry_run,
    }))
}

#[tauri::command]
pub async fn templates_run_group(
    state: State<'_, Arc<AppState>>,
    args: TemplatesIdArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let (rows, message) = run::run_group(&state, args.id.trim(), args.dry_run).await?;
    Ok(serde_json::json!({
        "ok": true,
        "rows_written": rows,
        "message": message,
        "dry_run": args.dry_run,
    }))
}

#[derive(Deserialize)]
pub struct TemplatesCancelArgs {
    /// `template` | `group`
    #[serde(default = "default_entity_template")]
    pub entity_type: String,
    pub id: String,
}

fn default_entity_template() -> String {
    "template".into()
}

#[tauri::command]
pub fn templates_cancel(
    state: State<'_, Arc<AppState>>,
    args: TemplatesCancelArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_session(&state)?;
    let id = args.id.trim();
    if id.is_empty() {
        return Err("Не указан id".into());
    }
    let et = args.entity_type.trim().to_ascii_lowercase();
    let key = if et == "group" {
        format!("group:{id}")
    } else {
        format!("template:{id}")
    };
    let ok = crate::templates::runs::cancel(&key);
    Ok(serde_json::json!({ "ok": ok }))
}

#[derive(Deserialize)]
pub struct TabNameArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct TabRenameArgs {
    #[serde(default = "default_wb")]
    pub marketplace: String,
    pub from: String,
    pub to: String,
}

#[tauri::command]
pub fn templates_select_tab(
    state: State<'_, Arc<AppState>>,
    args: TabNameArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let snap = store::select_tab(&args.marketplace, args.name.trim())?;
    Ok(serde_json::to_value(snap).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn templates_add_tab(
    state: State<'_, Arc<AppState>>,
    args: TabNameArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let snap = store::add_tab(&args.marketplace, args.name.trim())?;
    Ok(serde_json::to_value(snap).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn templates_rename_tab(
    state: State<'_, Arc<AppState>>,
    args: TabRenameArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let snap = store::rename_tab(&args.marketplace, args.from.trim(), args.to.trim())?;
    Ok(serde_json::to_value(snap).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn templates_delete_tab(
    state: State<'_, Arc<AppState>>,
    args: TabNameArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let snap = store::delete_tab(&args.marketplace, args.name.trim())?;
    Ok(serde_json::to_value(snap).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn templates_save_group(
    state: State<'_, Arc<AppState>>,
    group: TemplateGroup,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let saved = store::upsert_group(group)?;
    state.log.info(format!("Группа сохранена: {}", saved.name));
    Ok(serde_json::json!({ "group": saved }))
}

#[tauri::command]
pub fn templates_delete_group(
    state: State<'_, Arc<AppState>>,
    args: TemplatesIdArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let ok = store::delete_group(args.id.trim())?;
    if ok {
        state.log.info("Группа удалена");
    }
    Ok(serde_json::json!({ "ok": ok }))
}

#[derive(Deserialize)]
pub struct GroupExpandArgs {
    pub id: String,
    pub expanded: bool,
}

#[tauri::command]
pub fn templates_set_group_expanded(
    state: State<'_, Arc<AppState>>,
    args: GroupExpandArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    store::set_group_expanded(args.id.trim(), args.expanded)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct AddToGroupArgs {
    pub template_id: String,
    pub group_id: String,
}

#[tauri::command]
pub fn templates_add_to_group(
    state: State<'_, Arc<AppState>>,
    args: AddToGroupArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    store::add_to_group(args.template_id.trim(), args.group_id.trim())?;
    Ok(serde_json::json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct MoveTemplateArgs {
    pub template_id: String,
    pub direction: String,
    #[serde(default)]
    pub group_id: String,
}

#[tauri::command]
pub fn templates_move(
    state: State<'_, Arc<AppState>>,
    args: MoveTemplateArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    let gid = args.group_id.trim();
    crate::templates::order::move_template(
        args.template_id.trim(),
        args.direction.trim(),
        if gid.is_empty() { None } else { Some(gid) },
    )?;
    Ok(serde_json::json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct MoveGroupArgs {
    pub group_id: String,
    pub direction: String,
}

#[tauri::command]
pub fn templates_move_group(
    state: State<'_, Arc<AppState>>,
    args: MoveGroupArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    crate::templates::order::move_group(args.group_id.trim(), args.direction.trim())?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn templates_remove_from_group(
    state: State<'_, Arc<AppState>>,
    args: TemplatesIdArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    crate::templates::order::remove_from_group(args.id.trim())?;
    Ok(serde_json::json!({ "ok": true }))
}
