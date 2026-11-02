use super::wb_token;
use crate::core::paths;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvKeyRow {
    pub id: String,
    pub cabinet: String,
    pub marketplace: String,
    pub expires_at: String,
    pub env_key: String,
    pub value: String,
    #[serde(default)]
    pub ozon_seller_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvKeysSnapshot {
    pub path: String,
    pub rows: Vec<EnvKeyRow>,
}

/// Рабочий env: выбранный `*.env` рядом с exe, иначе `exe_dir/.env`.
pub fn default_path() -> PathBuf {
    let configured = crate::core::config::AppConfig::peek_env_file_path();
    resolve_path(&configured)
}

/// Сконфигурированный `*.env` рядом с exe, если валиден; иначе канон `.env`.
pub fn resolve_path(configured: &str) -> PathBuf {
    paths::purge_stray_env_copies();
    let c = configured.trim();
    if !c.is_empty() {
        if let Ok(p) = paths::ensure_env_beside_exe(std::path::Path::new(c)) {
            return p;
        }
    }
    paths::env_file_path()
}

/// Создать пустой `.env` по пути (кнопка «Создать файл ключей»).
pub fn create_empty(path: &PathBuf) -> Result<EnvKeysSnapshot, String> {
    ensure_file(path)?;
    list_rows(path)
}

pub fn list_rows(path: &PathBuf) -> Result<EnvKeysSnapshot, String> {
    if !path.is_file() {
        return Ok(EnvKeysSnapshot {
            path: path.display().to_string(),
            rows: Vec::new(),
        });
    }
    let mut map: BTreeMap<(String, String), (String, String)> = BTreeMap::new();

    for (key, value) in parse_env_file(path)? {
        if let Some(cab) = strip_prefix_ci(&key, "API_KEY_") {
            upsert(&mut map, cab, "WB", Some(value), None);
        } else if let Some(cab) = strip_prefix_ci(&key, "OZON_KEY_") {
            upsert(&mut map, cab, "OZON", Some(value), None);
        } else if let Some(cab) = strip_prefix_ci(&key, "OZON_ID_") {
            upsert(&mut map, cab, "OZON", None, Some(value));
        }
    }

    let rows = map
        .into_iter()
        .map(|((cabinet, marketplace), (api_key, seller))| {
            let is_ozon = marketplace.eq_ignore_ascii_case("OZON");
            let env_key = if is_ozon {
                format!("OZON_KEY_{cabinet}")
            } else {
                format!("API_KEY_{cabinet}")
            };
            let expires_at = if is_ozon {
                String::new()
            } else {
                wb_token::expires_label(&api_key)
            };
            EnvKeyRow {
                id: env_key.clone(),
                cabinet,
                marketplace,
                expires_at,
                env_key,
                value: api_key,
                ozon_seller_id: seller,
            }
        })
        .collect();

    Ok(EnvKeysSnapshot {
        path: path.display().to_string(),
        rows,
    })
}

pub fn save_rows(path: &PathBuf, rows: &[EnvKeyRow]) -> Result<EnvKeysSnapshot, String> {
    ensure_file(path)?;
    let existing = super::seal::read_plaintext(path)?;
    let existing_lines: Vec<String> = if existing.is_empty() {
        Vec::new()
    } else {
        existing.lines().map(str::to_string).collect()
    };

    let mut kept_keys = HashSet::new();
    let mut values: HashMap<String, String> = HashMap::new();
    let mut ozon_sellers: HashMap<String, String> = HashMap::new();

    for row in rows {
        let env_key = row.env_key.trim();
        if env_key.is_empty() {
            continue;
        }
        let key = env_key.to_string();
        kept_keys.insert(key.clone());
        values.insert(key, row.value.clone());
        if row.marketplace.eq_ignore_ascii_case("OZON")
            || env_key.starts_with("OZON_KEY_")
            || env_key.to_ascii_uppercase().starts_with("OZON_KEY_")
        {
            let cab = if row.cabinet.trim().is_empty() {
                strip_prefix_ci(env_key, "OZON_KEY_").unwrap_or_default()
            } else {
                row.cabinet.trim().to_string()
            };
            if !cab.is_empty() {
                ozon_sellers.insert(cab, row.ozon_seller_id.trim().to_string());
            }
        }
    }

    let kept_ozon_ids: HashSet<String> = ozon_sellers
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(cab, _)| format!("OZON_ID_{cab}"))
        .collect();

    let mut output = Vec::new();
    let mut seen = HashSet::new();

    for line in existing_lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || !t.contains('=') {
            output.push(line);
            continue;
        }
        let Some((raw_key, _)) = t.split_once('=') else {
            output.push(line);
            continue;
        };
        let key = raw_key.trim();
        if starts_with_ci(key, "API_KEY_") || starts_with_ci(key, "OZON_KEY_") {
            if !kept_keys.contains(key) {
                continue;
            }
            let val = values.get(key).cloned().unwrap_or_default();
            output.push(format!("{key}={val}"));
            seen.insert(key.to_string());
        } else if starts_with_ci(key, "OZON_ID_") {
            if !kept_ozon_ids.contains(key) {
                continue;
            }
            let cab = &key["OZON_ID_".len()..];
            let val = ozon_sellers.get(cab).cloned().unwrap_or_default();
            output.push(format!("{key}={val}"));
            seen.insert(key.to_string());
        } else {
            output.push(line);
        }
    }

    for key in &kept_keys {
        if seen.contains(key) {
            continue;
        }
        let val = values.get(key).cloned().unwrap_or_default();
        output.push(format!("{key}={val}"));
        seen.insert(key.clone());
    }

    for (cab, seller) in &ozon_sellers {
        if seller.is_empty() {
            continue;
        }
        let id_key = format!("OZON_ID_{cab}");
        if seen.contains(&id_key) {
            continue;
        }
        output.push(format!("{id_key}={seller}"));
    }

    let mut text = output.join("\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    super::seal::write_plaintext(path, &text)?;
    list_rows(path)
}

/// Копия текущего `.env` (тот же KEY=VALUE) в выбранный путь.
pub fn export_plaintext(src: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    if !src.is_file() {
        return Err("Файл .env не найден".into());
    }
    let plain = super::seal::read_plaintext(src)?;
    super::seal::write_plaintext(dest, &plain)
}

/// Все WB-токены за одно чтение `.env`: ключ — cabinet lower-case → Api-Key.
pub fn load_wb_tokens(path: &PathBuf) -> Result<HashMap<String, String>, String> {
    let snap = list_rows(path)?;
    let mut map = HashMap::new();
    for r in snap.rows {
        if !r.marketplace.eq_ignore_ascii_case("WB") {
            continue;
        }
        let token = r.value.trim().to_string();
        if token.is_empty() {
            continue;
        }
        map.insert(r.cabinet.trim().to_ascii_lowercase(), token);
    }
    Ok(map)
}

/// Все Ozon-пары за одно чтение `.env`: ключ — cabinet lower-case → `(Client-Id, Api-Key)`.
pub fn load_ozon_pairs(path: &PathBuf) -> Result<HashMap<String, (String, String)>, String> {
    let snap = list_rows(path)?;
    let mut map = HashMap::new();
    for r in snap.rows {
        if !r.marketplace.eq_ignore_ascii_case("OZON") {
            continue;
        }
        let client_id = r.ozon_seller_id.trim().to_string();
        let api_key = r.value.trim().to_string();
        if client_id.is_empty() || api_key.is_empty() {
            continue;
        }
        map.insert(r.cabinet.trim().to_ascii_lowercase(), (client_id, api_key));
    }
    Ok(map)
}

fn ensure_file(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if !path.exists() {
        super::seal::write_plaintext(path, "")?;
    }
    Ok(())
}

fn parse_env_file(path: &PathBuf) -> Result<Vec<(String, String)>, String> {
    let raw = super::seal::read_plaintext(path)?;
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim().to_string();
        if key.is_empty() {
            continue;
        }
        let mut value = v.trim().to_string();
        if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
            value = value[1..value.len() - 1].to_string();
        }
        out.push((key, value));
    }
    Ok(out)
}

fn upsert(
    map: &mut BTreeMap<(String, String), (String, String)>,
    cabinet: String,
    marketplace: &str,
    api_key: Option<String>,
    seller: Option<String>,
) {
    let cab = cabinet.trim().to_string();
    if cab.is_empty() {
        return;
    }
    let key = (cab, marketplace.to_string());
    let cur = map.get(&key).cloned().unwrap_or_default();
    map.insert(
        key,
        (
            api_key.unwrap_or(cur.0),
            seller.unwrap_or(cur.1),
        ),
    );
}

fn strip_prefix_ci(s: &str, prefix: &str) -> Option<String> {
    if s.len() < prefix.len() {
        return None;
    }
    if !s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        return None;
    }
    Some(s[prefix.len()..].to_string())
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn roundtrip_wb_and_ozon() {
        let dir = env::temp_dir().join(format!("ur_env_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env");
        let rows = vec![
            EnvKeyRow {
                id: "API_KEY_Shop".into(),
                cabinet: "Shop".into(),
                marketplace: "WB".into(),
                expires_at: String::new(),
                env_key: "API_KEY_Shop".into(),
                value: "tok".into(),
                ozon_seller_id: String::new(),
            },
            EnvKeyRow {
                id: "OZON_KEY_Oz".into(),
                cabinet: "Oz".into(),
                marketplace: "OZON".into(),
                expires_at: String::new(),
                env_key: "OZON_KEY_Oz".into(),
                value: "oz-tok".into(),
                ozon_seller_id: "123".into(),
            },
        ];
        let saved = save_rows(&path, &rows).unwrap();
        assert_eq!(saved.rows.len(), 2);
        let on_disk = fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("API_KEY_Shop=tok"));
        assert!(on_disk.contains("OZON_KEY_Oz=oz-tok"));
        assert!(on_disk.contains("OZON_ID_Oz=123"));
        assert!(!crate::core::env::seal::is_sealed_blob(&on_disk));
        let export = dir.join("backup.env");
        export_plaintext(&path, &export).unwrap();
        let exported = fs::read_to_string(&export).unwrap();
        assert!(exported.contains("API_KEY_Shop=tok"));
        let _ = fs::remove_dir_all(&dir);
    }
}
