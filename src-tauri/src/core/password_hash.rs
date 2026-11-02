//! Пароли в листе Users: argon2id. Старый plaintext при успешном входе перезаписывается.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Хэш для записи в ячейку B (PHC-строка `$argon2id$…`).
pub fn hash_password(plain: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// `(ok, needs_rehash)` — needs_rehash = хранилось plaintext или устаревший формат.
pub fn verify_password(plain: &str, stored: &str) -> (bool, bool) {
    let stored = stored.trim();
    if stored.is_empty() {
        return (false, false);
    }
    if stored.starts_with("$argon2") {
        let parsed = match PasswordHash::new(stored) {
            Ok(p) => p,
            Err(_) => return (false, false),
        };
        let ok = Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok();
        return (ok, false);
    }
    // Legacy plaintext
    (stored == plain, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_hash() {
        let h = hash_password("secret1").unwrap();
        assert!(h.starts_with("$argon2"));
        let (ok, rehash) = verify_password("secret1", &h);
        assert!(ok && !rehash);
        let (bad, _) = verify_password("other", &h);
        assert!(!bad);
    }

    #[test]
    fn plaintext_legacy() {
        let (ok, rehash) = verify_password("abc", "abc");
        assert!(ok && rehash);
        let (bad, _) = verify_password("abc", "xyz");
        assert!(!bad);
    }
}
