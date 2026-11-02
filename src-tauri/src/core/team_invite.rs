//! Код команды UR3: workspace + Service Account под паролем Admin.
//! Argon2id (как пароли Users) + AES-256-GCM; OAuth в код не входит.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};

const PREFIX: &str = "UR3.";
const MIN_PASSPHRASE: usize = 8;
/// Как Argon2::default() (m=19 MiB, t=2, p=1).
const KDF_M_COST: u32 = 19456;
const KDF_T_COST: u32 = 2;
const KDF_P_COST: u32 = 1;
const SALT_LEN: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitePayload {
    pub v: u32,
    pub workspace_id: String,
    #[serde(default)]
    pub title: String,
    pub service_account: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvelopeV3 {
    v: u32,
    workspace_id: String,
    #[serde(default)]
    title: String,
    salt: String,
    m: u32,
    t: u32,
    p: u32,
    nonce: String,
    ct: String,
}

pub fn encode(
    workspace_id: &str,
    title: &str,
    service_account_raw: &str,
    passphrase: &str,
) -> Result<String, String> {
    validate_passphrase(passphrase)?;
    let sa: Value = serde_json::from_str(service_account_raw.trim())
        .map_err(|e| format!("Service Account JSON: {e}"))?;
    let id = workspace_id.trim();
    if id.is_empty() {
        return Err("Нет workspace id".into());
    }
    let sa_bytes = serde_json::to_vec(&sa).map_err(|e| e.to_string())?;
    let compressed = deflate_bytes(&sa_bytes)?;

    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(passphrase.as_bytes(), &salt, KDF_M_COST, KDF_T_COST, KDF_P_COST)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, compressed.as_ref())
        .map_err(|_| "Не удалось зашифровать код команды".to_string())?;

    let env = EnvelopeV3 {
        v: 3,
        workspace_id: id.to_string(),
        title: title.trim().to_string(),
        salt: URL_SAFE_NO_PAD.encode(salt),
        m: KDF_M_COST,
        t: KDF_T_COST,
        p: KDF_P_COST,
        nonce: URL_SAFE_NO_PAD.encode(nonce_bytes),
        ct: URL_SAFE_NO_PAD.encode(ct),
    };
    let json = serde_json::to_vec(&env).map_err(|e| e.to_string())?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json)))
}

pub fn decode(code: &str, passphrase: &str) -> Result<TeamInvitePayload, String> {
    let t = code.trim();
    if t.is_empty() {
        return Err("Пустой код команды".into());
    }
    if strip_prefix_ci(t, "UR1.").is_some() {
        return Err(
            "Устаревший формат кода (UR1). Попросите Admin скопировать новый код UR3.…".into(),
        );
    }
    if strip_prefix_ci(t, "UR2.").is_some() {
        return Err(
            "Устаревший формат кода (UR2). Попросите Admin скопировать новый код UR3.…".into(),
        );
    }
    let rest = strip_prefix_ci(t, PREFIX).ok_or_else(|| {
        "Некорректный код команды (ожидается UR3.… от Admin)".to_string()
    })?;
    validate_passphrase(passphrase)?;

    let raw = URL_SAFE_NO_PAD
        .decode(rest.trim())
        .map_err(|_| "Некорректный код команды (ожидается UR3.… от Admin)".to_string())?;
    let env: EnvelopeV3 =
        serde_json::from_slice(&raw).map_err(|e| format!("Код команды: {e}"))?;
    if env.v != 3 {
        return Err(format!("Неподдерживаемая версия кода: {}", env.v));
    }
    if env.workspace_id.trim().is_empty() {
        return Err("В коде нет workspace id".into());
    }

    let salt = URL_SAFE_NO_PAD
        .decode(env.salt.trim())
        .map_err(|_| "Некорректный код команды (salt)".to_string())?;
    let nonce_bytes = URL_SAFE_NO_PAD
        .decode(env.nonce.trim())
        .map_err(|_| "Некорректный код команды (nonce)".to_string())?;
    let ct = URL_SAFE_NO_PAD
        .decode(env.ct.trim())
        .map_err(|_| "Некорректный код команды (ct)".to_string())?;
    if nonce_bytes.len() != 12 {
        return Err("Некорректный код команды (nonce length)".into());
    }

    let key = derive_key(
        passphrase.as_bytes(),
        &salt,
        env.m.max(1),
        env.t.max(1),
        env.p.max(1),
    )?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let compressed = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| "Неверный пароль кода команды или повреждённый код".to_string())?;
    let json = inflate_bytes(&compressed)?;
    let sa: Value =
        serde_json::from_slice(&json).map_err(|e| format!("Service Account в коде: {e}"))?;

    Ok(TeamInvitePayload {
        v: 3,
        workspace_id: env.workspace_id,
        title: env.title,
        service_account: sa,
    })
}

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.len() < MIN_PASSPHRASE {
        return Err(format!(
            "Пароль кода команды минимум {MIN_PASSPHRASE} символов"
        ));
    }
    Ok(())
}

fn derive_key(
    password: &[u8],
    salt: &[u8],
    m: u32,
    t: u32,
    p: u32,
) -> Result<[u8; 32], String> {
    let params = Params::new(m, t, p, Some(32)).map_err(|e| e.to_string())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|e| e.to_string())?;
    Ok(key)
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() < prefix.len() {
        return None;
    }
    let (head, rest) = s.split_at(prefix.len());
    if head.eq_ignore_ascii_case(prefix) {
        Some(rest)
    } else {
        None
    }
}

fn deflate_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(data).map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

fn inflate_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut dec = DeflateDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|_| "Некорректный код команды (сжатие)".to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sa() -> &'static str {
        r#"{"type":"service_account","project_id":"p","private_key_id":"k","private_key":"-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n","client_email":"a@p.iam.gserviceaccount.com","client_id":"1","token_uri":"https://oauth2.googleapis.com/token"}"#
    }

    #[test]
    fn roundtrip_ur3() {
        let code = encode("spreadsheet123", "WS", sample_sa(), "password1").unwrap();
        assert!(code.starts_with("UR3."), "{code}");
        assert!(!code.contains("private_key"));
        let p = decode(&code, "password1").unwrap();
        assert_eq!(p.workspace_id, "spreadsheet123");
        assert_eq!(p.service_account["client_email"], "a@p.iam.gserviceaccount.com");
    }

    #[test]
    fn wrong_passphrase_fails() {
        let code = encode("spreadsheet123", "WS", sample_sa(), "password1").unwrap();
        assert!(decode(&code, "password2").is_err());
    }

    #[test]
    fn rejects_ur2() {
        let err = decode("UR2.deadbeef", "password1").unwrap_err();
        assert!(err.contains("UR2") || err.contains("Устаревший"), "{err}");
    }

    #[test]
    fn short_passphrase_rejected() {
        assert!(encode("id", "t", sample_sa(), "short").is_err());
    }
}
