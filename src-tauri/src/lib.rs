mod app_state;
mod auth;
mod bridge;
mod calculator;
mod core;
mod scripts;
mod templates;
mod updates;

use app_state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::new());
    templates::scheduler::spawn(state.clone());
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            calculator::run_log::set_app_handle(app.handle().clone());
            calculator::set_team_sync_app_handle(app.handle().clone());
            templates::runs::set_app_handle(app.handle().clone());
            auth::set_app_handle(app.handle().clone());
            // Тихий прогрев Google user OAuth (refresh access; без браузера на старте).
            tauri::async_runtime::spawn(async {
                crate::core::google_oauth::warm_user_token().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bridge::app_version,
            bridge::i18n_get_locale,
            bridge::i18n_set_locale,
            bridge::logs_get,
            bridge::logs_clear,
            bridge::auth_status,
            bridge::auth_setup_status,
            bridge::auth_save_oauth_client,
            bridge::auth_save_service_account,
            bridge::auth_create_workspace_service_account,
            bridge::auth_replace_workspace_service_account,
            bridge::auth_connect_google,
            bridge::auth_switch_google_account,
            bridge::auth_register_admin,
            bridge::auth_join_workspace,
            bridge::auth_rename_workspace,
            bridge::auth_export_team_invite,
            bridge::auth_apply_team_invite,
            bridge::auth_team_oauth_get,
            bridge::auth_team_oauth_publish,
            bridge::auth_team_oauth_set_grants,
            bridge::auth_list_teams,
            bridge::auth_begin_new_team,
            bridge::auth_set_active_team,
            bridge::auth_switch_team,
            bridge::auth_invite_user,
            bridge::auth_list_team,
            bridge::auth_users_dashboard,
            bridge::access_list_presets,
            bridge::access_invalidate_users_cache,
            bridge::access_save_preset,
            bridge::access_delete_preset,
            bridge::access_set_user_preset,
            bridge::access_script_catalog,
            bridge::auth_login,
            bridge::auth_logout,
            bridge::auth_delete_user,
            bridge::auth_connect_member_oauth,
            bridge::auth_refresh_profile,
            bridge::auth_change_password,
            bridge::scripts_list,
            bridge::scripts_list_grouped,
            bridge::updates_check,
            bridge::updates_download_install,
            bridge::env_list_cabinet_names,
            bridge::env_save_rows,
            bridge::env_wb_token_info,
            bridge::env_check_keys,
            bridge::env_create_default,
            bridge::env_set_path,
            bridge::env_exe_dir,
            bridge::env_export_to,
            bridge::env_import_from,
            bridge::google_check_connection,
            bridge::google_list_sheets,
            bridge::spreadsheet_normalize,
            bridge::tabs_layout_get,
            bridge::tabs_layout_save,
            calculator::wb_calculator_get_snapshot,
            calculator::wb_calculator_get_snapshot_for,
            calculator::wb_calculator_save_settings,
            calculator::wb_calculator_set_triggers,
            calculator::wb_calculator_edit_table_link,
            calculator::wb_calculator_clear_table_links,
            calculator::wb_calculator_lookup_card,
            calculator::wb_calculator_warm_lookup_cache,
            calculator::wb_calculator_export_excel,
            calculator::wb_calculator_open_table,
            calculator::wb_calculator_append_profit_token,
            calculator::wb_calculator_clear_profit_formula,
            calculator::wb_calculator_export_config_file,
            calculator::wb_calculator_import_config,
            calculator::wb_calculator_import_config_file,
            calculator::wb_calculator_create_or_update_all,
            calculator::wb_calculator_refresh_table,
            calculator::wb_calculator_get_run_logs,
            calculator::wb_calculator_clear_run_logs,
            calculator::wb_calculator_list_team_configs,
            calculator::wb_calculator_publish_team_config,
            calculator::wb_calculator_set_default_team_config,
            calculator::wb_calculator_delete_team_config,
            calculator::wb_calculator_pull_team_config,
            bridge::auth_set_user_calc_config,
            templates::templates_snapshot,
            templates::templates_get,
            templates::templates_set_spreadsheet_title,
            templates::templates_set_spreadsheet_titles,
            templates::templates_save,
            templates::templates_delete,
            templates::templates_list_fields,
            templates::templates_run,
            templates::templates_run_group,
            templates::templates_cancel,
            templates::templates_select_tab,
            templates::templates_add_tab,
            templates::templates_rename_tab,
            templates::templates_delete_tab,
            templates::templates_save_group,
            templates::templates_delete_group,
            templates::templates_set_group_expanded,
            templates::templates_add_to_group,
            templates::templates_move,
            templates::templates_move_group,
            templates::templates_remove_from_group,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
