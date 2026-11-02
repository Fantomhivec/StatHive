use super::keys::{self as env_keys, EnvKeysSnapshot};
use super::wb_token;
use chrono::{Duration, Utc};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct KeyCheckLine {
    pub index: u32,
    pub text: String,
    pub tone: String,
    pub cabinet: String,
    pub marketplace: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyCheckResult {
    pub path: String,
    pub count: u32,
    pub wb_count: u32,
    pub ozon_count: u32,
    pub lines: Vec<KeyCheckLine>,
    pub warnings: Vec<String>,
    /// Файл `.env` существует по `path`.
    #[serde(default)]
    pub exists: bool,
}

/// Хотя бы один действующий ключ: непустой value; WB JWT не просрочен.
pub fn has_usable_key(path: &PathBuf) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(snap) = env_keys::list_rows(path) else {
        return false;
    };
    for row in &snap.rows {
        if row.value.trim().is_empty() {
            continue;
        }
        if row.marketplace.eq_ignore_ascii_case("WB") {
            match wb_token::decode(row.value.trim()) {
                Ok(info) if info.expired => continue,
                _ => return true,
            }
        } else if row.marketplace.eq_ignore_ascii_case("OZON") {
            return true;
        }
    }
    false
}

/// Текущий `.env` (выбранный `*.env` рядом с exe или канон). Без side-effect save.
pub fn has_usable_key_configured() -> bool {
    has_usable_key(&env_keys::default_path())
}

/// Проверка ключей .env (акцент на WB; OZON строки тоже видны).
/// Файл не создаёт: если нет — `exists: false`, пустые lines.
pub fn check_keys(path: &PathBuf) -> Result<KeyCheckResult, String> {
    if !path.is_file() {
        return Ok(KeyCheckResult {
            path: path.display().to_string(),
            count: 0,
            wb_count: 0,
            ozon_count: 0,
            lines: Vec::new(),
            warnings: Vec::new(),
            exists: false,
        });
    }
    let snap = env_keys::list_rows(path)?;
    let mut res = build_result(&snap);
    res.exists = true;
    Ok(res)
}

fn build_result(snap: &EnvKeysSnapshot) -> KeyCheckResult {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    let mut expired = Vec::new();
    let mut expiring = Vec::new();
    let mut index = 1u32;
    let mut wb_count = 0u32;
    let mut ozon_count = 0u32;

    for row in &snap.rows {
        if row.marketplace.eq_ignore_ascii_case("WB") {
            wb_count += 1;
            let (text, tone) = wb_line(index, &row.cabinet, &row.value);
            if tone == "danger" {
                expired.push(row.cabinet.clone());
            } else if tone == "warning" {
                expiring.push(row.cabinet.clone());
            }
            lines.push(KeyCheckLine {
                index,
                text,
                tone,
                cabinet: row.cabinet.clone(),
                marketplace: "WB".into(),
            });
            index += 1;
        } else if row.marketplace.eq_ignore_ascii_case("OZON") {
            ozon_count += 1;
            let has = !row.value.trim().is_empty();
            let sid = !row.ozon_seller_id.trim().is_empty();
            let (text, tone) = if has && sid {
                (
                    format!("{index}. OZON ({}) - ключ и seller_id найдены", row.cabinet),
                    "normal",
                )
            } else if has {
                (
                    format!("{index}. OZON ({}) - ключ есть, нет OZON_ID_*", row.cabinet),
                    "warning",
                )
            } else {
                (
                    format!("{index}. OZON ({}) - нет ключа", row.cabinet),
                    "warning",
                )
            };
            lines.push(KeyCheckLine {
                index,
                text,
                tone: tone.into(),
                cabinet: row.cabinet.clone(),
                marketplace: "OZON".into(),
            });
            index += 1;
        }
    }

    if !expired.is_empty() {
        warnings.push(format!(
            "Просроченные токены: {}. Обновите в личном кабинете WB.",
            expired.join(", ")
        ));
    }
    if !expiring.is_empty() {
        warnings.push(format!(
            "Токены истекают менее чем через 2 дня: {}.",
            expiring.join(", ")
        ));
    }

    KeyCheckResult {
        path: snap.path.clone(),
        count: lines.len() as u32,
        wb_count,
        ozon_count,
        lines,
        warnings,
        exists: true,
    }
}

fn wb_line(index: u32, cabinet: &str, token: &str) -> (String, String) {
    match wb_token::decode(token) {
        Err(_) => (
            format!("{index}. WB ({cabinet}) - ключ найден (не JWT / без срока)"),
            "normal".into(),
        ),
        Ok(info) if info.expired => (
            format!("{index}. WB ({cabinet}) - ключ найден, токен просрочен, обновите в ЛК"),
            "danger".into(),
        ),
        Ok(info) => {
            let mut tone = "normal".to_string();
            let mut type_part = String::new();
            if info.token_type.starts_with("Персональный") {
                tone = "personal".into();
                type_part = " [персональный токен]".into();
            } else if info.token_type.starts_with("Базовый") {
                tone = "basic".into();
                type_part = " [базовый токен]".into();
            }
            if is_expiring_soon(token) {
                tone = "warning".into();
            }
            let exp_part = if info.expires_at.is_empty() {
                "ключ найден".into()
            } else {
                format!("токен действителен до: {}", info.expires_at)
            };
            (
                format!("{index}. WB ({cabinet}){type_part} - {exp_part}"),
                tone,
            )
        }
    }
}

fn is_expiring_soon(token: &str) -> bool {
    let Some(exp) = wb_token::exp_unix(token) else {
        return false;
    };
    let now = Utc::now().timestamp();
    let soon = (Utc::now() + Duration::days(2)).timestamp();
    exp > now && exp <= soon
}
