//! At-rest для чувствительных файлов AppData команды.
//! AES-GCM + ключ в OS keyring (все платформы).

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rand::RngCore;
use std::fs;
use std::path::Path;

const PREFIX: &str = "STATHIVE_ATREST_V1.";
const SERVICE: &str = "StatHive";
const ACCOUNT: &str = "appdata-at-rest-v1";

/// Прочитать текст: расшифровать при префиксе, иначе plaintext (legacy).
pub fn read_text(path: &Path) -> Option<String> {
    read_text_result(path).ok().flatten()
}

/// Как `read_text`, но ошибки keyring/AES не глотаются (`Ok(None)` = нет файла/пусто).
pub fn read_text_result(path: &Path) -> Result<Option<String>, String> {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if let Some(rest) = t.strip_prefix(PREFIX) {
        return Ok(Some(unseal(rest)?));
    }
    Ok(Some(t.to_string()))
}

/// Записать с at-rest защитой. Без plaintext fallback: ошибка keyring/AES → Err.
pub fn write_text(path: &Path, plain: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body = plain.trim();
    let sealed = seal(body)?;
    let out = format!("{PREFIX}{sealed}");
    fs::write(path, out).map_err(|e| e.to_string())
}

fn seal(plain: &str) -> Result<String, String> {
    let bytes = protect(plain.as_bytes())?;
    Ok(B64.encode(bytes))
}

fn unseal(b64: &str) -> Result<String, String> {
    let bytes = B64
        .decode(b64.trim())
        .map_err(|e| format!("at-rest decode: {e}"))?;
    let plain = unprotect(&bytes)?;
    String::from_utf8(plain).map_err(|e| format!("at-rest utf8: {e}"))
}

fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    let key = master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut out = nonce_bytes.to_vec();
    let ct = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("aes seal: {e}"))?;
    out.extend_from_slice(&ct);
    Ok(out)
}

fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, String> {
    if sealed.len() < 12 + 16 {
        return Err("at-rest blob too short".into());
    }
    let key = master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let (nonce_bytes, ct) = sealed.split_at(12);
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|_| "at-rest decrypt failed".into())
}

fn master_key() -> Result<[u8; 32], String> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(s) => {
            let bytes = B64.decode(s.trim()).map_err(|e| e.to_string())?;
            if bytes.len() != 32 {
                return Err("bad keyring key length".into());
            }
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            Ok(k)
        }
        // Только отсутствие записи — создать ключ. Иначе не затирать: иначе
        // все STATHIVE_ATREST_V1.* на диске навсегда нечитаемы.
        Err(keyring::Error::NoEntry) => {
            let mut k = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut k);
            entry
                .set_password(&B64.encode(k))
                .map_err(|e| e.to_string())?;
            Ok(k)
        }
        Err(e) => Err(format!("keyring: {e}")),
    }
}
