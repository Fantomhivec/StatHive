//! Offline validators for WB script contracts (rates + JSON field paths).
//! This crate is NOT a dependency of `stathive` — run via `cargo test -p wb_script_contracts`.

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn catalog_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog")
}

pub fn fixtures_dir() -> PathBuf {
    catalog_dir().join("fixtures")
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScriptEntry {
    pub key: String,
    pub api: String,
    pub kind: String,
    #[serde(default)]
    pub api_updated: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RateStrategy {
    SlidingWindow,
    TokenBucket,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateSpec {
    pub strategy: String,
    #[serde(default)]
    pub max_requests: u32,
    #[serde(default)]
    pub window_ms: u64,
    #[serde(default)]
    pub tokens_per_sec: f64,
    #[serde(default)]
    pub capacity: f64,
    #[serde(default)]
    pub min_interval_ms: u64,
    #[serde(default)]
    pub pause_on_429_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct FieldReq {
    pub path: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Deserialize)]
pub struct FieldsFile {
    pub required: Vec<FieldReq>,
}

pub fn load_scripts() -> Result<Vec<ScriptEntry>, String> {
    read_json(&catalog_dir().join("scripts.json"))
}

pub fn load_rates(name: &str) -> Result<std::collections::HashMap<String, RateSpec>, String> {
    read_json(&catalog_dir().join(name))
}

pub fn load_fields(key: &str) -> Result<FieldsFile, String> {
    read_json(&catalog_dir().join("fields").join(format!("{key}.json")))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn rates_equal(a: &RateSpec, b: &RateSpec) -> bool {
    if a.strategy != b.strategy {
        return false;
    }
    match a.strategy.as_str() {
        "sliding_window" => {
            a.max_requests == b.max_requests
                && a.window_ms == b.window_ms
                && a.min_interval_ms == b.min_interval_ms
                && a.pause_on_429_ms == b.pause_on_429_ms
        }
        "token_bucket" => {
            almost_eq(a.tokens_per_sec, b.tokens_per_sec)
                && almost_eq(a.capacity, b.capacity)
                && a.min_interval_ms == b.min_interval_ms
                && a.pause_on_429_ms == b.pause_on_429_ms
        }
        _ => false,
    }
}

fn almost_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

pub fn assert_token_bucket_capacity_one(rates: &std::collections::HashMap<String, RateSpec>) -> Result<(), String> {
    for (k, r) in rates {
        if r.strategy == "token_bucket" && (r.capacity - 1.0).abs() > 1e-9 {
            return Err(format!("{k}: token_bucket capacity must be 1.0, got {}", r.capacity));
        }
    }
    Ok(())
}

/// Walk dotted path; `array` type accepts JSON array; nested objects via `.`.
pub fn path_exists_typed(root: &Value, path: &str, ty: &str) -> Result<(), String> {
    let mut cur = root;
    for part in path.split('.') {
        cur = cur
            .get(part)
            .ok_or_else(|| format!("missing path segment `{part}` in `{path}`"))?;
    }
    let ok = match ty {
        "string" => cur.is_string(),
        "integer" => cur.as_i64().is_some() || cur.as_u64().is_some(),
        "number" => cur.as_f64().is_some(),
        "boolean" => cur.is_boolean(),
        "array" => cur.is_array(),
        "object" => cur.is_object(),
        other => return Err(format!("unknown type `{other}`")),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("path `{path}`: expected type `{ty}`, got {cur}"))
    }
}

pub fn parse_groups_wb_keys(groups_rs: &str) -> Vec<String> {
    let start = groups_rs.find("groups.wb.productsEmoji").unwrap_or(0);
    let rest = &groups_rs[start..];
    let end = rest.find("groups.ozon").unwrap_or(rest.len());
    let chunk = &rest[..end];
    // Matches one-line and multi-line (`"key", "api", "date"`) triples after `(`.
    let mut out = Vec::new();
    let bytes = chunk.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'(' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'"' {
            i += 1;
            continue;
        }
        let start_k = j + 1;
        let mut k = start_k;
        while k < bytes.len() && bytes[k] != b'"' {
            k += 1;
        }
        if k >= bytes.len() {
            break;
        }
        let key = &chunk[start_k..k];
        // Script keys: snake_case or single token (cards_list, supplies, promotions).
        if key.is_empty()
            || key.contains('.')
            || key.starts_with("ozon_")
            || !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            i = k + 1;
            continue;
        }
        // Require a following string literal (api / date tuple), not a lone group label.
        let after = chunk[k + 1..].trim_start();
        if after.starts_with(',') {
            out.push(key.to_string());
        }
        i = k + 1;
    }
    out
}
