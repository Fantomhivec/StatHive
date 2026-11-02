use crate::app_state::AppState;
use crate::core::access::{self, can_run_script};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

fn session_user(state: &AppState) -> (String, bool) {
    state
        .auth
        .current_user()
        .map(|u| {
            let has_oauth = access::effective_has_oauth(&u.access_level, u.has_oauth);
            (u.access_level, has_oauth)
        })
        .unwrap_or_else(|| ("Denied".into(), false))
}

fn allow_script(state: &AppState, level: &str, has_oauth: bool, key: &str) -> bool {
    can_run_script(
        level,
        key,
        has_oauth,
        state.registry.is_member_ok(key),
    )
}

#[tauri::command]
pub fn scripts_list(state: State<'_, Arc<AppState>>) -> Vec<(String, String)> {
    let (level, has_oauth) = session_user(&state);
    state
        .registry
        .list()
        .into_iter()
        .filter(|(k, _)| allow_script(&state, &level, has_oauth, k))
        .collect()
}

#[derive(Deserialize)]
pub struct ScriptsGroupedArgs {
    #[serde(default = "default_wb_market")]
    pub marketplace: String,
}

fn default_wb_market() -> String {
    "wb".into()
}

#[tauri::command]
pub fn scripts_list_grouped(
    state: State<'_, Arc<AppState>>,
    args: ScriptsGroupedArgs,
) -> Result<serde_json::Value, String> {
    let (level, has_oauth) = session_user(&state);
    let mut groups = crate::scripts::groups::list_grouped(&state.registry, &args.marketplace);
    groups.retain_mut(|g| {
        g.scripts
            .retain(|s| allow_script(&state, &level, has_oauth, &s.key));
        !g.scripts.is_empty()
    });
    Ok(serde_json::json!({ "groups": groups }))
}

