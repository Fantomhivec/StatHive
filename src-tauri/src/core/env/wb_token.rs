use crate::core::sheet_dates;
use chrono::{DateTime, Local, Utc};
use serde_json::Value;

const CATEGORIES: &[(u32, &str)] = &[
    (1, "Контент"),
    (2, "Аналитика"),
    (3, "Цены и скидки"),
    (4, "Маркетплейс"),
    (5, "Статистика"),
    (6, "Продвижение"),
    (7, "Вопросы и отзывы"),
    (9, "Чат с покупателями"),
    (10, "Поставки"),
    (11, "Возвраты покупателями"),
    (12, "Документы"),
    (13, "Финансы"),
    (16, "Пользователи"),
    (30, "Только чтение (флаг)"),
];

#[derive(Debug, Clone)]
pub struct WbTokenInfo {
    pub token_type: String,
    pub acc: Option<i64>,
    pub for_value: Option<String>,
    pub test: bool,
    pub id: String,
    pub sid: String,
    pub read_only: bool,
    pub expires_at: String,
    pub expired: bool,
    pub categories: Vec<String>,
}

pub fn decode(token: &str) -> Result<WbTokenInfo, String> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() < 2 {
        return Err("Неверный формат JWT".into());
    }
    let bytes = b64url_decode(parts[1]).map_err(|_| "Не удалось декодировать payload".to_string())?;
    let payload: Value = serde_json::from_slice(&bytes).map_err(|_| "Payload не JSON".to_string())?;
    let acc = json_i64(payload.get("acc").or_else(|| payload.get("ent")));
    let for_value = json_str(payload.get("for"));
    let test = json_bool(payload.get("t")).unwrap_or(false);
    let exp = json_i64(payload.get("exp"));
    let (expires_at, expired) = match exp {
        Some(secs) => {
            let label = format_exp(secs);
            let expired = secs < Utc::now().timestamp();
            (label, expired)
        }
        None => (String::new(), false),
    };
    let mask = json_u64(payload.get("s"));
    let mut categories = Vec::new();
    let mut read_only = false;
    if let Some(mask) = mask {
        for (bit, name) in CATEGORIES {
            if (mask >> bit) & 1 == 1 {
                if *bit == 30 {
                    read_only = true;
                } else {
                    categories.push((*name).to_string());
                }
            }
        }
    }
    Ok(WbTokenInfo {
        token_type: token_type(acc, for_value.as_deref(), test),
        acc,
        for_value,
        test,
        id: json_str(payload.get("id")).unwrap_or_default(),
        sid: json_str(payload.get("sid")).unwrap_or_default(),
        read_only,
        expires_at,
        expired,
        categories,
    })
}

pub fn expires_label(token: &str) -> String {
    match decode(token) {
        Ok(info) if info.expires_at.is_empty() => "—".into(),
        Ok(info) if info.expired => format!("{} (истёк)", info.expires_at),
        Ok(info) => info.expires_at,
        Err(_) => "—".into(),
    }
}

/// Unix `exp` из JWT payload, если есть.
pub fn exp_unix(token: &str) -> Option<i64> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let bytes = b64url_decode(parts[1]).ok()?;
    let payload: Value = serde_json::from_slice(&bytes).ok()?;
    json_i64(payload.get("exp"))
}

pub fn info_text(cabinet: &str, info: &WbTokenInfo) -> String {
    let cats = if info.categories.is_empty() {
        "—".into()
    } else {
        info.categories
            .iter()
            .map(|c| format!("- {c}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "Кабинет: {cabinet}\nПлощадка: WB\nТип: {ty}\nacc/ent: {acc}\nfor: {for_v}\nТестовый: {test}\nID: {id}\nSID: {sid}\nТолько чтение: {ro}\nИстекает: {exp}\nКатегории:\n{cats}",
        ty = info.token_type,
        acc = info.acc.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
        for_v = info.for_value.clone().unwrap_or_else(|| "—".into()),
        test = if info.test { "да" } else { "нет" },
        id = dash(&info.id),
        sid = dash(&info.sid),
        ro = if info.read_only { "да" } else { "нет" },
        exp = if info.expires_at.is_empty() {
            "—".into()
        } else if info.expired {
            format!("{} (истёк)", info.expires_at)
        } else {
            info.expires_at.clone()
        },
    )
}

fn token_type(acc: Option<i64>, for_value: Option<&str>, test: bool) -> String {
    let _ = acc;
    let for_value = for_value.map(str::trim).filter(|s| !s.is_empty());
    match for_value {
        None if !test => "Базовый токен".into(),
        None => "Тестовый токен".into(),
        Some(v) if v.to_ascii_lowercase().contains("asid") => "Сервисный токен".into(),
        Some(v) if v.eq_ignore_ascii_case("self") => "Персональный токен".into(),
        Some(v) => format!("Персональный токен ({v})"),
    }
}

fn format_exp(secs: i64) -> String {
    let Some(dt) = DateTime::from_timestamp(secs, 0) else {
        return String::new();
    };
    sheet_dates::format_datetime(dt.with_timezone(&Local).naive_local())
}

fn dash(s: &str) -> &str {
    if s.is_empty() { "—" } else { s }
}

fn json_i64(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    v.as_i64()
        .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| v.as_f64().map(|n| n.floor() as i64))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn json_u64(v: Option<&Value>) -> Option<u64> {
    let v = v?;
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|n| u64::try_from(n).ok()))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn json_bool(v: Option<&Value>) -> Option<bool> {
    let v = v?;
    v.as_bool()
        .or_else(|| v.as_i64().map(|n| n != 0))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn json_str(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn b64url_decode(input: &str) -> Result<Vec<u8>, ()> {
    let mut s = input.replace('-', "+").replace('_', "/");
    match s.len() % 4 {
        0 => {}
        2 => s.push_str("=="),
        3 => s.push('='),
        _ => return Err(()),
    }
    b64_decode(&s)
}

fn b64_decode(input: &str) -> Result<Vec<u8>, ()> {
    fn val(c: u8) -> Result<u8, ()> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(()),
        }
    }
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|c| **c == b'=').count();
        if pad > 2 {
            return Err(());
        }
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = if pad < 2 { val(chunk[2])? } else { 0 };
        let d = if pad < 1 { val(chunk[3])? } else { 0 };
        let n = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | d as u32;
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
fn b64url_encode(data: &[u8]) -> String {
    const ALPH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | data[i + 2] as u32;
        out.push(ALPH[((n >> 18) & 63) as usize] as char);
        out.push(ALPH[((n >> 12) & 63) as usize] as char);
        out.push(ALPH[((n >> 6) & 63) as usize] as char);
        out.push(ALPH[(n & 63) as usize] as char);
        i += 3;
    }
    if i < data.len() {
        let rem = data.len() - i;
        let mut n = (data[i] as u32) << 16;
        if rem == 2 {
            n |= (data[i + 1] as u32) << 8;
        }
        out.push(ALPH[((n >> 18) & 63) as usize] as char);
        out.push(ALPH[((n >> 12) & 63) as usize] as char);
        if rem == 2 {
            out.push(ALPH[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        out.push('=');
    }
    out.trim_end_matches('=').replace('+', "-").replace('/', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_jwt() -> String {
        let payload = br#"{"exp":1893456000,"s":14,"acc":1,"id":"tok","sid":"seller"}"#;
        format!("e30.{}.x", b64url_encode(payload))
    }

    #[test]
    fn decodes_type_categories_and_exp() {
        let info = decode(&sample_jwt()).unwrap();
        assert_eq!(info.token_type, "Базовый токен");
        assert_eq!(info.categories, vec!["Контент", "Аналитика", "Цены и скидки"]);
        assert!(!info.read_only);
        assert_eq!(info.sid, "seller");
        assert!(!info.expires_at.is_empty());
        assert!(!info.expired);
    }

    #[test]
    fn rejects_plain_secret() {
        assert!(decode("not-a-jwt").is_err());
    }
}
