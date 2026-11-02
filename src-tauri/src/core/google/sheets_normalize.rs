//! Нормализация ячеек для инкрементальной записи (паритет IncrementalSheetWriter.NormalizeValue).

use crate::core::sheet_dates::try_normalize_date_for_compare;
use regex::Regex;
use std::sync::OnceLock;

/// Канон для сравнения ключей и ячеек (даты / URL / числа / формулы / пробелы).
pub fn normalize_value(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return String::new();
    }

    // ="..." → содержимое (="+*" ≡ +*)
    if s.len() >= 3 && s.starts_with("=\"") && s.ends_with('"') {
        s = s[2..s.len() - 1].to_string();
        if s.trim().is_empty() {
            return String::new();
        }
        s = s.trim().to_string();
    }

    if let Some(d) = try_normalize_date_for_compare(&s) {
        return d;
    }

    if let Some(u) = try_normalize_url(&s) {
        return u;
    }

    if let Some(n) = try_normalize_number(&s) {
        return n;
    }

    s.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

pub fn values_equal(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    normalize_value(left) == normalize_value(right)
}

fn try_normalize_number(raw: &str) -> Option<String> {
    let parse = |s: &str| -> Option<f64> {
        let n: f64 = s.parse().ok()?;
        if n.is_finite() {
            Some(n)
        } else {
            None
        }
    };

    let n = parse(raw).or_else(|| {
        // ru-RU: "1,5" без точки-тысяч
        if raw.contains(',') && !raw.contains('.') {
            parse(&raw.replace(',', "."))
        } else {
            None
        }
    })?;

    // Аналог decimal G29: без лишних нулей (Rust Display для 100.0 → "100").
    Some(format!("{n}"))
}

fn try_normalize_url(raw: &str) -> Option<String> {
    let mut candidate = raw.trim().to_string();
    if candidate.is_empty() {
        return None;
    }

    static HYPERLINK: OnceLock<Regex> = OnceLock::new();
    let re = HYPERLINK.get_or_init(|| {
        Regex::new(r#"(?i)^=\s*HYPERLINK\s*\(\s*\"([^\"]+)\""#).expect("hyperlink regex")
    });
    if let Some(caps) = re.captures(&candidate) {
        candidate = caps.get(1)?.as_str().trim().to_string();
    }

    let lower = candidate.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return None;
    }

    // scheme://host/path?query#frag — без полного URL crate.
    let (scheme, rest) = if lower.starts_with("https://") {
        ("https", &candidate[8..])
    } else {
        ("http", &candidate[7..])
    };

    let (authority_path, query_frag) = match rest.find(['?', '#']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };

    let (host, path) = match authority_path.find('/') {
        Some(i) => (&authority_path[..i], &authority_path[i..]),
        None => (authority_path, ""),
    };
    if host.is_empty() {
        return None;
    }

    let host = host.to_ascii_lowercase();
    let mut path = path.to_string();
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }

    Some(format!("{scheme}://{host}{path}{query_frag}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_equivalent() {
        assert!(values_equal("2026-10-27", "27.10.2026"));
        assert!(values_equal("27/10/2026", "27.10.2026"));
        assert!(values_equal(
            "2026-10-27T14:30:00",
            "27.10.2026 14:30:00"
        ));
    }

    #[test]
    fn numbers_equivalent() {
        assert!(values_equal("100", "100.0"));
        assert!(values_equal("1,5", "1.5"));
    }

    #[test]
    fn urls_equivalent() {
        // Host lowercased; path case preserved (как Uri in ); trailing / снят.
        assert!(values_equal("HTTPS://A/B/", "https://a/B"));
        assert!(values_equal("https://a/b/", "https://a/b"));
        assert!(values_equal(
            r#"=HYPERLINK("https://a/b","x")"#,
            "https://a/b"
        ));
    }

    #[test]
    fn formula_quote_and_whitespace() {
        assert!(values_equal(r#"="+*""#, "+*"));
        assert!(values_equal("Ab C", "abc"));
        assert!(values_equal("True", "true"));
    }

    #[test]
    fn empty_key_parts_join() {
        // BuildNormalizedKey: пустые части дают "||"
        let parts = [normalize_value(""), normalize_value("x")];
        let key = format!("{}||{}||", parts[0], parts[1]);
        assert_eq!(key, "||x||");
        assert!(!key.is_empty());
    }
}
