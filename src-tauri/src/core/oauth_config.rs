//! Локальный OAuth Desktop client (AppData / active team). Не коммитить в git.
//! Plaintext в AppData (как SA/token) — keyring на части ПК ломал auto-pull.
//! Также кэш «можно ли жать Проверить подключение» (грант TeamOAuth / Admin).

use crate::core::access::{self, can_manage_team};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

const ATREST_PREFIX: &str = "STATHIVE_ATREST_V1.";

fn path() -> PathBuf {
    crate::core::team_profile::active_dir().join("oauth_client.json")
}

fn write_plaintext(raw: &str) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&p, raw.trim()).map_err(|e| e.to_string())
}

/// Сырой JSON Google Desktop OAuth (`installed` / `web`).
pub fn load_raw() -> Option<String> {
    load_raw_result().ok().flatten()
}

/// Plaintext; readable sealed → migrate to plaintext; unreadable sealed → `Ok(None)`.
pub fn load_raw_result() -> Result<Option<String>, String> {
    let p = path();
    let disk = match fs::read_to_string(&p) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let t = disk.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t.starts_with(ATREST_PREFIX) {
        match crate::core::secure_store::read_text_result(&p) {
            Ok(Some(plain)) => {
                let _ = write_plaintext(&plain);
                Ok(Some(plain))
            }
            Ok(None) => Ok(None),
            // Старый blob с другим ключом keyring — как «нет client», нужен pull/paste.
            Err(_) => Ok(None),
        }
    } else {
        Ok(Some(t.to_string()))
    }
}

pub fn is_configured() -> bool {
    load_raw().is_some_and(|r| parse_client_id_secret(&r).is_ok())
}

/// Укороченный `client_id` для экрана Admin (без секрета).
pub fn client_id_hint() -> Option<String> {
    let raw = load_raw()?;
    let (id, _, _) = parse_client_id_secret(&raw).ok()?;
    if id.len() <= 16 {
        return Some(id);
    }
    Some(format!("{}…{}", &id[..10], &id[id.len().saturating_sub(6)..]))
}

pub fn save_raw(raw: &str) -> Result<(), String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("Пустой OAuth JSON".into());
    }
    let _ = parse_client_id_secret(t)?;
    write_plaintext(t)
}

/// Удалить локальный Desktop client (shared PC: логин без гранта).
pub fn clear_local() {
    let _ = fs::remove_file(path());
    clear_check_google_cap();
}

pub fn parse_client_id_secret(raw: &str) -> Result<(String, String, String), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("OAuth JSON: {e}"))?;
    let installed = v
        .get("installed")
        .or_else(|| v.get("web"))
        .ok_or_else(|| "OAuth JSON: нет installed/web".to_string())?;
    let client_id = installed
        .get("client_id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let client_secret = installed
        .get("client_secret")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err("OAuth JSON: пустые client_id/client_secret".into());
    }
    let token_uri = installed
        .get("token_uri")
        .and_then(|x| x.as_str())
        .unwrap_or("https://oauth2.googleapis.com/token")
        .trim()
        .to_string();
    Ok((client_id, client_secret, token_uri))
}

/// `project_id` из Desktop/Web OAuth JSON (нужен для IAM create SA).
pub fn project_id() -> Option<String> {
    let raw = load_raw()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let from_root = v
        .get("project_id")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if from_root.is_some() {
        return from_root.map(str::to_string);
    }
    v.get("installed")
        .or_else(|| v.get("web"))
        .and_then(|b| b.get("project_id"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn check_google_cache() -> &'static Mutex<Option<(String, String, bool)>> {
    static M: Mutex<Option<(String, String, bool)>> = Mutex::new(None);
    &M
}

pub fn set_check_google_cap(team: &str, login: &str, allowed: bool) {
    if let Ok(mut g) = check_google_cache().lock() {
        *g = Some((team.to_string(), login.trim().to_string(), allowed));
    }
}

pub fn clear_check_google_cap() {
    if let Ok(mut g) = check_google_cache().lock() {
        *g = None;
    }
}

/// Admin / кэш гранта TeamOAuth. Наличие локального client **не** даёт доступ без гранта
/// (shared PC: чужой pull не открывает кнопку следующему логину).
pub fn can_check_google(access_level: &str, login: &str) -> bool {
    if access::is_admin_role(access_level) || can_manage_team(access_level) {
        return true;
    }
    let team = crate::core::team_profile::active_id().unwrap_or_default();
    let login = login.trim();
    if let Ok(g) = check_google_cache().lock() {
        if let Some((t, l, ok)) = g.as_ref() {
            if t == &team && l.eq_ignore_ascii_case(login) {
                return *ok;
            }
        }
    }
    false
}
