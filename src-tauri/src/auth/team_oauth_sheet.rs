//! Лист TeamOAuth на таблице Users: Desktop client JSON + гранты логинов.
//! Main Admin публикует/редактирует; pull — Admin всегда, остальные по гранту.
//! Участники без своего OAuth могут пользоваться командным Desktop client (тестовый user).

use crate::core::access::{self, can_manage_team};
use crate::core::i18n::{t, tf};
use crate::core::oauth_config;
use crate::core::google_sheets::GoogleSheetsClient;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub const SHEET_NAME: &str = "TeamOAuth";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamOauthGrantRow {
    pub login: String,
    pub display_name: String,
    pub allowed: bool,
    /// Builtin Admin — всегда true, UI disabled.
    pub locked_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamOauthState {
    pub has_client: bool,
    pub client_id_hint: Option<String>,
    pub grants: Vec<TeamOauthGrantRow>,
}

async fn sa_client() -> Result<(GoogleSheetsClient, String), String> {
    let sid = super::sheets_users::spreadsheet_id()?;
    let client = super::sheets_users::auth_client().await?;
    Ok((client, sid))
}

pub async fn ensure_sheet(client: &GoogleSheetsClient, sid: &str) -> Result<(), String> {
    client.ensure_sheet_exists(sid, SHEET_NAME).await
}

/// A1=client json. A3=Login, B3=Allowed. A4+=grants.
async fn read_client_json(client: &GoogleSheetsClient, sid: &str) -> Result<Option<String>, String> {
    let rows = client
        .read_values(sid, &format!("'{SHEET_NAME}'!A1:A1"))
        .await?;
    let raw = rows
        .first()
        .and_then(|r| r.first())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(raw)
}

fn grants_from_matrix(rows: &[Vec<String>]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        let login = row.first().map(|s| s.trim()).unwrap_or("");
        if login.is_empty() || login.eq_ignore_ascii_case("Login") {
            continue;
        }
        let allowed = row
            .get(1)
            .map(|s| {
                let t = s.trim().to_ascii_lowercase();
                t == "1" || t == "true" || t == "да" || t == "yes"
            })
            .unwrap_or(false);
        if allowed {
            out.push(login.to_string());
        }
    }
    out
}

async fn read_grant_logins(client: &GoogleSheetsClient, sid: &str) -> Result<Vec<String>, String> {
    let rows = client
        .read_values(sid, &format!("'{SHEET_NAME}'!A4:B200"))
        .await?;
    Ok(grants_from_matrix(&rows))
}

fn client_id_hint_from_json(raw: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| {
            v.get("installed")
                .or_else(|| v.get("web"))
                .and_then(|o| o.get("client_id"))
                .and_then(|c| c.as_str())
                .map(|s| {
                    let s = s.trim();
                    if s.len() <= 12 {
                        s.to_string()
                    } else {
                        format!("{}…{}", &s[..6], &s[s.len() - 4..])
                    }
                })
        })
}

pub async fn publish_client(raw_json: &str) -> Result<(), String> {
    let raw = raw_json.trim();
    if raw.is_empty() {
        return Err(t("errors.auth.saveOAuthFirst"));
    }
    let _: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("OAuth JSON: {e}"))?;
    let (client, sid) = sa_client().await?;
    ensure_sheet(&client, &sid).await?;
    client
        .write_values(
            &sid,
            &format!("'{SHEET_NAME}'!A1:B3"),
            vec![
                vec![raw.to_string()],
                vec![],
                vec!["Login".into(), "Allowed".into()],
            ],
        )
        .await?;
    Ok(())
}

pub async fn get_state() -> Result<TeamOauthState, String> {
    let (client, sid) = sa_client().await?;
    ensure_sheet(&client, &sid).await?;
    let client_json = read_client_json(&client, &sid).await?;
    let granted = read_grant_logins(&client, &sid).await?;
    let users = super::sheets_users::list_users(&client, &sid).await?;

    let mut grants = Vec::new();
    for u in users {
        let is_admin = access::is_admin_role(&u.access_level);
        let allowed = is_admin
            || granted
                .iter()
                .any(|g| g.eq_ignore_ascii_case(&u.login));
        grants.push(TeamOauthGrantRow {
            login: u.login.clone(),
            display_name: u.display_name(),
            allowed,
            locked_on: is_admin,
        });
    }

    Ok(TeamOauthState {
        has_client: client_json.is_some(),
        client_id_hint: client_json.as_ref().and_then(|j| client_id_hint_from_json(j)),
        grants,
    })
}

pub async fn set_grants(allowed_logins: &[String]) -> Result<(), String> {
    let (client, sid) = sa_client().await?;
    ensure_sheet(&client, &sid).await?;
    let users = super::sheets_users::list_users(&client, &sid).await?;
    let mut rows = vec![vec!["Login".into(), "Allowed".into()]];
    for u in users {
        if access::is_admin_role(&u.access_level) {
            continue;
        }
        let allowed = allowed_logins
            .iter()
            .any(|g| g.eq_ignore_ascii_case(&u.login));
        rows.push(vec![
            u.login.clone(),
            if allowed { "true".into() } else { "false".into() },
        ]);
    }
    let clear_rows: Vec<Vec<String>> = (0..80)
        .map(|_| vec![String::new(), String::new()])
        .collect();
    client
        .write_values(&sid, &format!("'{SHEET_NAME}'!A3:B100"), clear_rows)
        .await?;
    client
        .write_values(&sid, &format!("'{SHEET_NAME}'!A3"), rows)
        .await?;
    Ok(())
}

pub fn login_may_pull(access_level: &str, login: &str, granted: &[String]) -> bool {
    if can_manage_team(access_level) || access::is_admin_role(access_level) {
        return true;
    }
    granted
        .iter()
        .any(|g| g.eq_ignore_ascii_case(login.trim()))
}

fn map_missing_sheet_err(e: String) -> String {
    let low = e.to_ascii_lowercase();
    if low.contains("unable to parse range") || low.contains("unable to parse") {
        t("errors.auth.teamOauthNotPublished")
    } else {
        e
    }
}

pub async fn pull_if_allowed(access_level: &str, login: &str) -> Result<String, String> {
    let (client, sid) = sa_client().await?;
    // Без ensure_sheet (spreadsheets.get) — лишний GET ловит 429.
    // Один batchGet: A1 (client) + A4:B (grants).
    let ranges = [
        format!("'{SHEET_NAME}'!A1:A1"),
        format!("'{SHEET_NAME}'!A4:B200"),
    ];
    let batch = client
        .read_values_batch(&sid, &[&ranges[0], &ranges[1]])
        .await
        .map_err(map_missing_sheet_err)?;
    let client_rows = batch.first().cloned().unwrap_or_default();
    let grant_rows = batch.get(1).cloned().unwrap_or_default();
    let granted = grants_from_matrix(&grant_rows);
    if !login_may_pull(access_level, login, &granted) {
        return Err(t("errors.auth.teamOauthDenied"));
    }
    let raw = client_rows
        .first()
        .and_then(|r| r.first())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| t("errors.auth.teamOauthNotPublished"))?;
    oauth_config::save_raw(&raw)?;
    Ok(oauth_config::client_id_hint().unwrap_or_default())
}

fn auto_pull_tried_team() -> &'static Mutex<Option<String>> {
    static M: Mutex<Option<String>> = Mutex::new(None);
    &M
}

fn mark_auto_pull_tried(team: &str) {
    if let Ok(mut g) = auto_pull_tried_team().lock() {
        *g = Some(team.to_string());
    }
}

/// Сброс при смене команды — иначе новый слот не подтянет TeamOAuth.
pub fn reset_auto_pull_gate() {
    if let Ok(mut g) = auto_pull_tried_team().lock() {
        *g = None;
    }
    crate::core::oauth_config::clear_check_google_cap();
}

/// Shared PC: без гранта не оставлять client/token на диске.
fn revoke_local_team_oauth_files() {
    oauth_config::clear_local();
    crate::core::google_oauth::clear_cached_token();
}

/// Обновить кэш гранта (Sheets). Вызывать из tabs_layout / после auto_pull.
/// При **подтверждённом** отсутствии гранта — удалить `oauth_client.json` и token
/// (не при сетевой ошибке чтения листа).
pub async fn refresh_can_check_google(access_level: &str, login: &str) -> bool {
    let team = crate::core::team_profile::active_id().unwrap_or_default();
    if access::is_admin_role(access_level) || can_manage_team(access_level) {
        oauth_config::set_check_google_cap(&team, login, true);
        return true;
    }
    match sa_client().await {
        Ok((client, sid)) => match read_grant_logins(&client, &sid).await {
            Ok(granted) => {
                let allowed = login_may_pull(access_level, login, &granted);
                oauth_config::set_check_google_cap(&team, login, allowed);
                if !allowed {
                    revoke_local_team_oauth_files();
                }
                allowed
            }
            Err(_) => {
                oauth_config::set_check_google_cap(&team, login, false);
                false
            }
        },
        Err(_) => {
            oauth_config::set_check_google_cap(&team, login, false);
            false
        }
    }
}

fn is_auto_pull_soft_skip(err: &str) -> bool {
    err == t("errors.auth.teamOauthDenied")
        || err == t("errors.auth.teamOauthNotPublished")
        || err.contains("Нет доступа к Google")
        || err.contains("Нет доступа к OAuth")
        || err.contains("ещё не опубликовал")
}

/// После логина / старта: один раз на активную команду.
/// Без гранта — client с диска снять; с грантом — pull, если client ещё нет.
pub async fn auto_pull_if_needed(
    access_level: &str,
    login: &str,
    log: &crate::core::log::AppLog,
) {
    let team = crate::core::team_profile::active_id().unwrap_or_default();
    // Сначала ACL: чужой client на shared PC не должен пережить логин без гранта.
    if oauth_config::is_configured() {
        let allowed = refresh_can_check_google(access_level, login).await;
        if !allowed {
            log.info(tf(
                "log.auth.teamOauthAutoPullSkip",
                &[("error", &t("errors.auth.teamOauthDenied"))],
            ));
            return;
        }
        if !crate::core::google_oauth::has_cached_token() {
            log.info(t("log.auth.teamOauthNeedCheck"));
        }
        return;
    }
    if let Ok(g) = auto_pull_tried_team().lock() {
        if g.as_ref() == Some(&team) {
            return;
        }
    }
    match pull_if_allowed(access_level, login).await {
        Ok(_) => {
            mark_auto_pull_tried(&team);
            oauth_config::set_check_google_cap(&team, login, true);
            log_pull_ok(log, login);
        }
        Err(e) if e == t("errors.auth.teamOauthDenied") => {
            mark_auto_pull_tried(&team);
            revoke_local_team_oauth_files();
            oauth_config::set_check_google_cap(&team, login, false);
            log.info(tf("log.auth.teamOauthAutoPullSkip", &[("error", &e)]));
        }
        Err(e) if is_auto_pull_soft_skip(&e) => {
            mark_auto_pull_tried(&team);
            log.info(tf("log.auth.teamOauthAutoPullSkip", &[("error", &e)]));
            let _ = refresh_can_check_google(access_level, login).await;
        }
        Err(e) => {
            // 429 и прочие — не помечаем: повторим на следующем tabs_layout / входе.
            log.warn(tf("log.auth.teamOauthAutoPullFailed", &[("error", &e)]));
        }
    }
}

pub async fn publish_from_local() -> Result<(), String> {
    let raw = oauth_config::load_raw().ok_or_else(|| t("errors.auth.saveOAuthFirst"))?;
    publish_client(&raw).await
}

pub fn log_publish_ok(log: &crate::core::log::AppLog) {
    log.info(t("log.auth.teamOauthPublished"));
}

pub fn log_pull_ok(log: &crate::core::log::AppLog, login: &str) {
    log.info(tf("log.auth.teamOauthPulled", &[("login", login)]));
}
