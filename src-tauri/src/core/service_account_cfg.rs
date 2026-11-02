//! Локальный Google Service Account JSON (AppData / active team).
//! Plaintext в AppData (вход участника не зависит от keyring). Не коммитить в git.

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const ATREST_PREFIX: &str = "STATHIVE_ATREST_V1.";

fn path() -> PathBuf {
    crate::core::team_profile::active_dir().join("service_account.json")
}

fn write_plaintext(raw: &str) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&p, raw.trim()).map_err(|e| e.to_string())
}

pub fn load_raw() -> Option<String> {
    load_raw_result().ok().flatten()
}

/// Plaintext; readable sealed → migrate to plaintext; unreadable sealed → `Ok(None)` (снова код).
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
            // Старый blob с другим ключом keyring — как «нет SA», нужен повторный UR3.
            Err(_) => Ok(None),
        }
    } else {
        Ok(Some(t.to_string()))
    }
}

pub fn is_configured() -> bool {
    load_raw().is_some_and(|r| parse_key(&r).is_ok())
}

pub fn path_exists() -> bool {
    path().is_file()
}

pub fn save_raw(raw: &str) -> Result<(), String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("Пустой Service Account JSON".into());
    }
    let _ = parse_key(t)?;
    write_plaintext(t)
}

pub fn client_email() -> Option<String> {
    load_raw()
        .and_then(|r| parse_key(&r).ok())
        .map(|(email, _, _)| email)
}

/// `(client_email, private_key_pem, token_uri)`.
pub fn parse_key(raw: &str) -> Result<(String, String, String), String> {
    let v: Value = serde_json::from_str(raw).map_err(|e| format!("Service Account JSON: {e}"))?;
    let typ = v
        .get("type")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    if !typ.is_empty() && !typ.eq_ignore_ascii_case("service_account") {
        return Err("Ожидается type=service_account".into());
    }
    let client_email = v
        .get("client_email")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    // В JSON от Google `\n` уже развёрнуты serde; при ручной вставке часто остаются литералы `\\n`.
    let private_key = v
        .get("private_key")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .replace("\\n", "\n");
    if client_email.is_empty() || private_key.is_empty() {
        return Err("Service Account JSON: пустые client_email/private_key".into());
    }
    let token_uri = v
        .get("token_uri")
        .and_then(|x| x.as_str())
        .unwrap_or("https://oauth2.googleapis.com/token")
        .trim()
        .to_string();
    Ok((client_email, private_key, token_uri))
}
