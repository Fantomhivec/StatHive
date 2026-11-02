use crate::app_state::AppState;
use crate::auth::{AuthSetupStatus, TeamUserRow, UserInfo, UsersDashboard};
use crate::bridge::access_cmds::PresetDto;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn app_version() -> String {
    crate::core::app_version::current().into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsGetArgs {
    #[serde(default)]
    pub known_revision: u64,
}

#[tauri::command]
pub fn logs_get(
    state: State<'_, Arc<AppState>>,
    args: Option<LogsGetArgs>,
) -> Result<serde_json::Value, String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    let known = args.map(|a| a.known_revision).unwrap_or(0);
    let (lines, revision) = state.log.snapshot_if_changed(known);
    Ok(serde_json::json!({
        "revision": revision,
        "unchanged": lines.is_none(),
        "lines": lines.unwrap_or_default(),
    }))
}

#[tauri::command]
pub fn logs_clear(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let _ = crate::templates::access_gate::require_user(&state)?;
    state.log.clear();
    Ok(())
}

#[tauri::command]
pub fn auth_status(state: State<'_, Arc<AppState>>) -> Option<UserInfo> {
    state.auth.ensure_activity_timer();
    state.auth.current_user()
}

#[tauri::command]
pub fn auth_setup_status(state: State<'_, Arc<AppState>>) -> AuthSetupStatus {
    state.auth.setup_status()
}

#[derive(Deserialize)]
pub struct SaveOauthClientArgs {
    pub json: String,
}

/// Первый setup без workspace, иначе только main Admin.
/// Не keysCheck: иначе участник после logout перезапишет OAuth-клиент машины.
fn require_setup_or_main_admin(state: &AppState) -> Result<(), String> {
    let workspace_ready = crate::core::workspace_cfg::is_configured();
    match state.auth.current_user() {
        None => {
            if workspace_ready {
                Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
            } else {
                Ok(())
            }
        }
        Some(u) if u.is_authorized => {
            if crate::core::access::is_admin_role(&u.access_level) {
                Ok(())
            } else {
                Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
            }
        }
        _ => Err(crate::core::i18n::t("errors.auth.notAuthorized")),
    }
}

/// Desktop OAuth JSON: setup без workspace, иначе только main Admin (как UI).
/// Не keysCheck: участник не перезаписывает командный client на shared PC.
fn require_oauth_client_edit(state: &AppState) -> Result<(), String> {
    require_setup_or_main_admin(state)
}

#[tauri::command]
pub fn auth_save_oauth_client(
    state: State<'_, Arc<AppState>>,
    args: SaveOauthClientArgs,
) -> Result<(), String> {
    require_oauth_client_edit(&state)?;
    state.auth.save_oauth_client(&args.json)
}

#[derive(Deserialize)]
pub struct SaveServiceAccountArgs {
    pub json: String,
}

#[tauri::command]
pub async fn auth_save_service_account(
    state: State<'_, Arc<AppState>>,
    args: SaveServiceAccountArgs,
) -> Result<String, String> {
    state.auth.save_service_account(&args.json).await
}

#[tauri::command]
pub async fn auth_create_workspace_service_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.auth.create_workspace_service_account().await
}

#[tauri::command]
pub async fn auth_replace_workspace_service_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state.auth.replace_workspace_service_account().await
}

#[tauri::command]
pub async fn auth_connect_google(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    require_oauth_client_edit(&state)?;
    state.auth.connect_google().await
}

#[tauri::command]
pub async fn auth_switch_google_account(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    require_oauth_client_edit(&state)?;
    state.auth.switch_google_account().await
}

#[derive(Deserialize)]
pub struct RegisterAdminArgs {
    pub login: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub workspace_title: String,
}

#[tauri::command]
pub async fn auth_register_admin(
    state: State<'_, Arc<AppState>>,
    args: RegisterAdminArgs,
) -> Result<UserInfo, String> {
    state
        .auth
        .register_admin(
            &args.login,
            &args.password,
            &args.display_name,
            &args.workspace_title,
        )
        .await
}

#[derive(Deserialize)]
pub struct JoinWorkspaceArgs {
    pub spreadsheet_id: String,
    #[serde(default)]
    pub title: String,
}

#[tauri::command]
pub fn auth_join_workspace(
    state: State<'_, Arc<AppState>>,
    args: JoinWorkspaceArgs,
) -> Result<(), String> {
    require_setup_or_main_admin(&state)?;
    state.auth.join_workspace(&args.spreadsheet_id, &args.title)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorkspaceArgs {
    pub title: String,
}

#[tauri::command]
pub async fn auth_rename_workspace(
    state: State<'_, Arc<AppState>>,
    args: RenameWorkspaceArgs,
) -> Result<String, String> {
    state.auth.rename_workspace(&args.title).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTeamInviteArgs {
    pub passphrase: String,
}

#[tauri::command]
pub fn auth_export_team_invite(
    state: State<'_, Arc<AppState>>,
    args: ExportTeamInviteArgs,
) -> Result<String, String> {
    state.auth.export_team_invite_code(&args.passphrase)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTeamInviteArgs {
    pub code: String,
    pub passphrase: String,
}

#[tauri::command]
pub async fn auth_apply_team_invite(
    state: State<'_, Arc<AppState>>,
    args: ApplyTeamInviteArgs,
) -> Result<(), String> {
    // Хаб / logout: без сессии можно добавить другую команду. В сессии — только Admin.
    match state.auth.current_user() {
        Some(u) if u.is_authorized => {
            if !crate::core::access::can_manage_team(&u.access_level) {
                return Err(crate::core::i18n::t("errors.auth.needAdminAuth"));
            }
        }
        None => {}
        _ => return Err(crate::core::i18n::t("errors.auth.notAuthorized")),
    }
    state
        .auth
        .apply_team_invite_code(&args.code, &args.passphrase)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamIdArgs {
    pub id: String,
}

#[tauri::command]
pub fn auth_list_teams() -> Vec<crate::core::team_profile::TeamIndexEntry> {
    crate::core::team_profile::list_teams()
}

#[tauri::command]
pub fn auth_begin_new_team(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    // Хаб «Настроить новый OAuth» до логина: пустой слот new-*, без сессии Admin.
    state.auth.begin_new_team()
}

#[tauri::command]
pub fn auth_set_active_team(
    state: State<'_, Arc<AppState>>,
    args: TeamIdArgs,
) -> Result<(), String> {
    state.auth.set_active_team(&args.id)
}

#[tauri::command]
pub async fn auth_switch_team(
    state: State<'_, Arc<AppState>>,
    args: TeamIdArgs,
) -> Result<crate::auth::SwitchTeamResult, String> {
    state.auth.switch_team(&args.id).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteUserArgs {
    pub login: String,
    pub password: String,
    pub display_name: String,
    pub google_email: String,
    /// Обязательный id пресета прав (лист «Пресеты»).
    pub preset_id: String,
    /// Id конфигурации CalcTeam (пусто → default).
    #[serde(default)]
    pub calc_config_id: String,
}

#[tauri::command]
pub async fn auth_invite_user(
    state: State<'_, Arc<AppState>>,
    args: InviteUserArgs,
) -> Result<crate::auth::TeamUserRow, String> {
    let login = args.login.clone();
    state.auth
        .invite_user(
            &args.login,
            &args.password,
            &args.display_name,
            &args.google_email,
            &args.preset_id,
            &args.calc_config_id,
        )
        .await
        .map_err(|e| {
            state.log.error(crate::core::i18n::tf(
                "log.auth.inviteFailed",
                &[("login", &login), ("error", &e)],
            ));
            e
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserCalcConfigArgs {
    pub row_index: u32,
    pub calc_config_id: String,
}

#[tauri::command]
pub async fn auth_set_user_calc_config(
    state: State<'_, Arc<AppState>>,
    args: SetUserCalcConfigArgs,
) -> Result<(), String> {
    let gen = crate::auth::team::next_assign_generation(args.row_index);
    let row = args.row_index;
    state
        .auth
        .set_user_calc_config(row, &args.calc_config_id, gen)
        .await
        .map_err(|e| {
            state.log.error(crate::core::i18n::tf(
                "log.auth.inviteFailed",
                &[("login", &row.to_string()), ("error", &e)],
            ));
            e
        })
}

#[tauri::command]
pub async fn auth_list_team(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<TeamUserRow>, String> {
    state.auth.list_team().await
}

#[derive(serde::Deserialize)]
pub struct UsersDashboardArgs {
    #[serde(default)]
    pub force: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersDashboardDto {
    pub team: Vec<TeamUserRow>,
    pub presets: Vec<PresetDto>,
    pub workspace_id: Option<String>,
    pub workspace_title: String,
    pub service_account_configured: bool,
    pub service_account_email: Option<String>,
}

impl From<UsersDashboard> for UsersDashboardDto {
    fn from(d: UsersDashboard) -> Self {
        Self {
            team: d.team,
            presets: d.presets.into_iter().map(PresetDto::from).collect(),
            workspace_id: d.workspace_id,
            workspace_title: d.workspace_title,
            service_account_configured: d.service_account_configured,
            service_account_email: d.service_account_email,
        }
    }
}

#[tauri::command]
pub async fn auth_users_dashboard(
    state: State<'_, Arc<AppState>>,
    args: UsersDashboardArgs,
) -> Result<UsersDashboardDto, String> {
    let dash = state.auth.users_dashboard(args.force).await?;
    Ok(UsersDashboardDto::from(dash))
}

#[tauri::command]
pub async fn auth_login(
    state: State<'_, Arc<AppState>>,
    login: String,
    password: String,
    google_email: Option<String>,
) -> Result<UserInfo, String> {
    state
        .auth
        .login(&login, &password, google_email.as_deref().unwrap_or(""))
        .await
}

#[tauri::command]
pub async fn auth_logout(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.auth.logout().await;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserArgs {
    pub row_index: u32,
}

#[tauri::command]
pub async fn auth_delete_user(
    state: State<'_, Arc<AppState>>,
    args: DeleteUserArgs,
) -> Result<(), String> {
    state.auth.delete_user(args.row_index).await
}

#[tauri::command]
pub async fn auth_connect_member_oauth(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<UserInfo, String> {
    use tauri::Emitter;
    let user = state.auth.connect_member_oauth().await?;
    state.auth.ensure_presets_loaded().await;
    let any_ok =
        crate::templates::access_gate::any_member_ok_in_scope(&state, &user.access_level);
    let oauth = crate::core::access::effective_has_oauth(&user.access_level, user.has_oauth);
    let can_check =
        crate::core::oauth_config::can_check_google(&user.access_level, &user.login);
    let layout = crate::core::tabs_layout::get_layout(
        &user.login,
        &user.access_level,
        oauth,
        any_ok,
        can_check,
    );
    let _ = app.emit("tabsLayout.changed", &layout);
    Ok(user)
}

#[tauri::command]
pub async fn auth_refresh_profile(
    state: State<'_, Arc<AppState>>,
) -> Result<UserInfo, String> {
    state.auth.refresh_profile().await
}

#[derive(Deserialize)]
pub struct ChangePasswordArgs {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[tauri::command]
pub async fn auth_change_password(
    state: State<'_, Arc<AppState>>,
    args: ChangePasswordArgs,
) -> Result<serde_json::Value, String> {
    match state
        .auth
        .change_password(
            &args.current_password,
            &args.new_password,
            &args.confirm_password,
        )
        .await
    {
        Ok(()) => Ok(serde_json::json!({ "success": true })),
        Err(e) => Ok(serde_json::json!({ "success": false, "error": e })),
    }
}

fn require_manage_team(state: &AppState) -> Result<crate::auth::UserInfo, String> {
    let u = state
        .auth
        .current_user()
        .filter(|u| u.is_authorized)
        .ok_or_else(|| crate::core::i18n::t("errors.auth.notAuthorized"))?;
    if crate::core::access::can_manage_team(&u.access_level) {
        Ok(u)
    } else {
        Err(crate::core::i18n::t("errors.auth.needAdminAuth"))
    }
}

#[tauri::command]
pub async fn auth_team_oauth_get(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::auth::team_oauth_sheet::TeamOauthState, String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::get_state().await
}

#[tauri::command]
pub async fn auth_team_oauth_publish(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::publish_from_local().await?;
    crate::auth::team_oauth_sheet::log_publish_ok(&state.log);
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamOauthSetGrantsArgs {
    pub allowed_logins: Vec<String>,
}

#[tauri::command]
pub async fn auth_team_oauth_set_grants(
    state: State<'_, Arc<AppState>>,
    args: TeamOauthSetGrantsArgs,
) -> Result<(), String> {
    require_manage_team(&state)?;
    crate::auth::team_oauth_sheet::set_grants(&args.allowed_logins).await
}

