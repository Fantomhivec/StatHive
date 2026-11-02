use regex::Regex;
use std::sync::OnceLock;

/// Извлекает spreadsheetId из чистого ID или URL Google Sheets (паритет SpreadsheetIdHelper).
pub fn try_extract(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    static RE_URL: OnceLock<Regex> = OnceLock::new();
    let re_url = RE_URL.get_or_init(|| {
        Regex::new(r"(?i)spreadsheets/d/([a-zA-Z0-9\-_]+)").expect("spreadsheet url regex")
    });
    if let Some(c) = re_url.captures(input) {
        let id = c.get(1)?.as_str();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    static RE_SEG: OnceLock<Regex> = OnceLock::new();
    let re_seg = RE_SEG.get_or_init(|| {
        Regex::new(r"([a-zA-Z0-9\-_]{15,})[/?#]").expect("spreadsheet segment regex")
    });
    if let Some(c) = re_seg.captures(input) {
        let id = c.get(1)?.as_str();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    static RE_PLAIN: OnceLock<Regex> = OnceLock::new();
    let re_plain = RE_PLAIN.get_or_init(|| Regex::new(r"^[a-zA-Z0-9\-_]+$").expect("plain id"));
    if re_plain.is_match(input) {
        return Some(input.to_string());
    }
    None
}

pub fn normalize(input: &str) -> (String, bool) {
    let raw = input.trim().to_string();
    match try_extract(&raw) {
        Some(id) if id != raw => (id, true),
        Some(id) => (id, false),
        None => (raw, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_from_url() {
        let id = try_extract("https://docs.google.com/spreadsheets/d/1AbC_def-123/edit#gid=0").unwrap();
        assert_eq!(id, "1AbC_def-123");
    }

    #[test]
    fn accepts_plain_id() {
        assert_eq!(try_extract("1AbC_def-123xyz").unwrap(), "1AbC_def-123xyz");
    }
}
