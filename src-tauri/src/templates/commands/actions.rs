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
    AddToGroupArgs, GroupExpandArgs, MoveGroupArgs, MoveTemplateArgs, TabNameArgs, TabRenameArgs,
    TemplatesCancelArgs, TemplatesIdArgs,
};
use crate::app_state::AppState;
use crate::templates::access_gate;
use crate::templates::model::TemplateGroup;
use crate::templates::{run, store};
use std::sync::Arc;
use tauri::State;

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

#[tauri::command]
pub fn templates_set_group_expanded(
    state: State<'_, Arc<AppState>>,
    args: GroupExpandArgs,
) -> Result<serde_json::Value, String> {
    access_gate::require_any_script(&state)?;
    store::set_group_expanded(args.id.trim(), args.expanded)?;
    Ok(serde_json::json!({ "ok": true }))
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
