//! Пресеты прав / назначение пользователю (Admin или флаг users).

use crate::app_state::AppState;
use crate::auth::sheets_users::{self, auth_client, spreadsheet_id};
use crate::core::access::{
    self, delete_preset, ensure_access_sheets, list_presets, resolve_preset_id, save_preset,
    AccessMatrix, PresetDef, ALL_APP_FLAGS, ALL_CALC_FLAGS, APP_FLAG_KEYS_CHECK, APP_FLAG_USERS,
};
use crate::scripts::build_registry;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::State;

fn require_users_tab(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if !access::can_users_tab(&u.access_level) {
        return Err(crate::core::i18n::t("errors.auth.usersTabDenied"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixDto {
    pub all: bool,
    pub keys: Vec<String>,
}

impl From<&AccessMatrix> for MatrixDto {
    fn from(m: &AccessMatrix) -> Self {
        match m {
            AccessMatrix::All => Self {
                all: true,
                keys: vec![],
            },
            AccessMatrix::Keys(set) => {
                let mut keys: Vec<_> = set.iter().cloned().collect();
                keys.sort();
                Self { all: false, keys }
            }
        }
    }
}

impl From<MatrixDto> for AccessMatrix {
    fn from(d: MatrixDto) -> Self {
        if d.all {
            AccessMatrix::All
        } else {
            AccessMatrix::Keys(d.keys.into_iter().collect())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDto {
    pub id: String,
    pub title: String,
    pub calculator: bool,
    pub scripts: MatrixDto,
    #[serde(default)]
    pub calc_access: Vec<String>,
    #[serde(default)]
    pub app_access: Vec<String>,
}

impl From<PresetDef> for PresetDto {
    fn from(p: PresetDef) -> Self {
        let mut calc_access: Vec<_> = p.calc_access.into_iter().collect();
        calc_access.sort();
        let mut app_access: Vec<_> = p.app_access.into_iter().collect();
        app_access.sort();
        Self {
            id: p.id,
            title: p.title,
            calculator: p.calculator,
            scripts: MatrixDto::from(&p.scripts),
            calc_access,
            app_access,
        }
    }
}

async fn client_sid() -> Result<(crate::core::google_sheets::GoogleSheetsClient, String), String> {
    let sid = spreadsheet_id()?;
    let client = auth_client().await?;
    ensure_access_sheets(&client, &sid).await?;
    Ok((client, sid))
}

#[tauri::command]
pub async fn access_list_presets(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<PresetDto>, String> {
    require_users_tab(&state)?;
    if access::presets_cache_ready() {
        return Ok(access::cached_presets()
            .into_iter()
            .map(PresetDto::from)
            .collect());
    }
    let (client, sid) = client_sid().await?;
    let presets = list_presets(&client, &sid).await?;
    access::set_presets_cache(presets.clone());
    Ok(presets.into_iter().map(PresetDto::from).collect())
}

#[tauri::command]
pub fn access_invalidate_users_cache(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    require_users_tab(&state)?;
    access::invalidate_users_ui_cache();
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePresetArgs {
    /// Пусто при создании — id из title.
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub calculator: bool,
    pub scripts: MatrixDto,
    #[serde(default)]
    pub calc_access: Vec<String>,
    #[serde(default)]
    pub app_access: Vec<String>,
}

fn normalize_calc_access(flags: Vec<String>, _calculator: bool) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();
    for f in flags {
        let n = f.trim().to_ascii_lowercase();
        if ALL_CALC_FLAGS.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
            set.insert(n);
        }
    }
    // Empty = no calculator (USER_ACCESS). Legacy sheet rows still expand on *read*
    // via parse_calc_access_cell(..., calculator_legacy=true).
    set
}

fn normalize_app_access(flags: Vec<String>) -> HashSet<String> {
    let mut set = HashSet::new();
    for f in flags {
        let n = f.trim().to_ascii_lowercase();
        if n == "keyscheck" || n == "keys_check" || n == "keys" {
            set.insert(APP_FLAG_KEYS_CHECK.to_string());
        } else if n == "users" || n == "user" || n == "team" {
            set.insert(APP_FLAG_USERS.to_string());
        } else if ALL_APP_FLAGS.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
            set.insert(n);
        }
    }
    set
}

#[tauri::command]
pub async fn access_save_preset(
    state: State<'_, Arc<AppState>>,
    args: SavePresetArgs,
) -> Result<PresetDto, String> {
    require_users_tab(&state)?;
    let (client, sid) = client_sid().await?;
    let calc_access = normalize_calc_access(args.calc_access, args.calculator);
    let preset = PresetDef {
        id: args.id,
        title: args.title,
        calculator: args.calculator,
        scripts: AccessMatrix::from(args.scripts),
        calc_access,
        app_access: normalize_app_access(args.app_access),
    };
    let saved = save_preset(&client, &sid, preset).await?;
    state.log.info(format!("Пресет сохранён: {}", saved.id));
    // Тот же процесс: если живая сессия на этом пресете — сразу caps/layout.
    if let Some(cur) = state.auth.current_user().filter(|u| u.is_authorized) {
        if cur.access_level.eq_ignore_ascii_case(&saved.id) {
            let _ = crate::auth::emit_session_updated(&cur);
        }
    }
    Ok(PresetDto::from(saved))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetIdArgs {
    pub preset_id: String,
}

#[tauri::command]
pub async fn access_delete_preset(
    state: State<'_, Arc<AppState>>,
    args: PresetIdArgs,
) -> Result<(), String> {
    require_users_tab(&state)?;
    let (client, sid) = client_sid().await?;
    delete_preset(&client, &sid, &args.preset_id).await?;
    if let Err(e) = state
        .auth
        .clear_users_after_preset_delete(&args.preset_id)
        .await
    {
        state.log.warn(format!(
            "Пресет удалён ({}), очистка пользователей: {e}",
            args.preset_id
        ));
    }
    state.log.info(format!("Пресет удалён: {}", args.preset_id));
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserPresetArgs {
    pub row_index: u32,
    pub preset_id: String,
}

#[tauri::command]
pub async fn access_set_user_preset(
    state: State<'_, Arc<AppState>>,
    args: SetUserPresetArgs,
) -> Result<(), String> {
    require_users_tab(&state)?;
    if args.row_index < 2 {
        return Err(crate::core::i18n::t("errors.auth.invalidUserRowIndex"));
    }
    let (client, sid) = client_sid().await?;
    let id = resolve_preset_id(&client, &sid, &args.preset_id).await?;
    let users = sheets_users::list_users(&client, &sid).await?;
    let target = users
        .iter()
        .find(|u| u.row_index == args.row_index)
        .ok_or_else(|| crate::core::i18n::t("errors.auth.userNotFound"))?;
    if access::is_admin_role(&target.access_level) {
        return Err(crate::core::i18n::t("errors.auth.cannotChangeAdminPreset"));
    }
    let old_level = target.access_level.clone();
    sheets_users::write_access_level(&client, &sid, args.row_index, &id).await?;
    access::invalidate_team_cache();
    state
        .auth
        .resync_drive_for_user_row(args.row_index, &old_level)
        .await
        .map_err(|e| {
            state.log.warn(format!(
                "Drive после смены пресета row={}: {e}",
                args.row_index
            ));
            format!("Пресет сохранён, но Drive reader не обновлён: {e}")
        })?;
    state.log.info(format!(
        "Пользователю row={} назначен пресет {}",
        args.row_index, id
    ));
    // Своя строка в этом процессе — без ожидания activity timer.
    if state
        .auth
        .current_user()
        .filter(|u| u.is_authorized && u.row_index == args.row_index)
        .is_some()
    {
        if let Err(e) = state.auth.refresh_profile().await {
            state.log.warn(format!(
                "Пресет назначен row={}, refresh сессии: {e}",
                args.row_index
            ));
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCatalogItem {
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub member_ok: bool,
}

#[tauri::command]
pub fn access_script_catalog(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ScriptCatalogItem>, String> {
    require_users_tab(&state)?;
    let mut items: Vec<_> = build_registry()
        .list_with_meta()
        .into_iter()
        .map(|(key, title, member_ok)| ScriptCatalogItem {
            key,
            title,
            member_ok,
        })
        .collect();
    items.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(items)
}
