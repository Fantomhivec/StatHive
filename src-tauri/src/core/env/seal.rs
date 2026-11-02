//! One-shot recovery of `STATHIVE_ENV_V1.` blobs written by a short-lived
//! at-rest experiment. Working `.env` is always classic KEY=VALUE plaintext.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::fs;
use std::path::Path;

const MAGIC: &str = "STATHIVE_ENV_V1.";
#[cfg(not(test))]
const KEYRING_SERVICE: &str = "StatHive";
#[cfg(not(test))]
const KEYRING_ACCOUNT: &str = "env-kek-v1";

pub fn is_sealed_blob(raw: &str) -> bool {
    raw.trim_start().starts_with(MAGIC)
}

/// Read `.env` as KEY=VALUE. If a legacy sealed blob is found, try to decrypt
/// once and rewrite plaintext on disk (so notepad / team copies stay readable).
pub fn read_plaintext(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Ok(String::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if !is_sealed_blob(trimmed) {
        return Ok(raw);
    }
    let plain = open_blob(trimmed).map_err(|_| {
        "Не удалось расшифровать .env (устаревший зашифрованный формат). \
Удалите файл рядом с приложением и нажмите «Создать файл ключей», \
либо вставьте ключи вручную в новый .env."
            .to_string()
    })?;
    write_plaintext(path, &plain)?;
    Ok(plain)
}

pub fn write_plaintext(path: &Path, plaintext: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let mut text = plaintext.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    fs::write(path, text).map_err(|e| e.to_string())
}

fn open_blob(blob: &str) -> Result<String, String> {
    let rest = blob
        .trim()
        .strip_prefix(MAGIC)
        .ok_or_else(|| "bad magic".to_string())?;
    let packed = URL_SAFE_NO_PAD
        .decode(rest.as_bytes())
        .map_err(|_| "bad b64".to_string())?;
    if packed.len() < 12 + 16 {
        return Err("short".into());
    }
    let (nonce_bytes, ct) = packed.split_at(12);
    let key = kek_from_keyring()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plain = cipher.decrypt(nonce, ct).map_err(|_| "decrypt".to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

#[cfg(test)]
fn kek_from_keyring() -> Result<[u8; 32], String> {
    Ok([0x42; 32])
}

#[cfg(not(test))]
fn kek_from_keyring() -> Result<[u8; 32], String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|e| e.to_string())?;
    let stored = entry.get_password().map_err(|e| e.to_string())?;
    let bytes = hex::decode(stored.trim()).map_err(|_| "bad kek".to_string())?;
    if bytes.len() != 32 {
        return Err("bad kek len".into());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}
