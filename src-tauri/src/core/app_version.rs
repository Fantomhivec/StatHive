//! Версия приложения из файла `VERSION` в корне репо (формат как .NET: `0.0.0.1`, любое число сегментов).
//! Cargo/npm требуют 3-сегментный semver — он отдельно; для UI и обновлений — этот файл.

use std::cmp::Ordering;

/// Текущая версия приложения (без префикса `v`).
pub fn current() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../VERSION"))
        .trim()
        .trim_start_matches(['v', 'V'])
}

/// Нормализация тега/строки: убрать `v`, оставить цифры и точки.
pub fn normalize(raw: &str) -> String {
    let s = raw.trim().trim_start_matches(['v', 'V']).trim();
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    cleaned.trim_matches('.').to_string()
}

/// Сравнение как `System.Version`: `a.b.c.d` по сегментам слева направо (недостающие = 0).
pub fn is_newer(remote: &str, local: &str) -> bool {
    compare(&normalize(remote), &normalize(local)) == Ordering::Greater
}

fn parse_parts(s: &str) -> Vec<u64> {
    if s.is_empty() {
        return vec![0];
    }
    s.split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect()
}

fn compare(a: &str, b: &str) -> Ordering {
    let pa = parse_parts(a);
    let pb = parse_parts(b);
    let n = pa.len().max(pb.len());
    for i in 0..n {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_part_newer() {
        assert!(is_newer("0.0.0.2", "0.0.0.1"));
        assert!(!is_newer("0.0.0.1", "0.0.0.1"));
        assert!(!is_newer("0.0.0.1", "0.0.0.2"));
    }

    #[test]
    fn unequal_length() {
        assert!(is_newer("1.0.0.1", "1.0.0"));
        assert!(is_newer("2", "1.9.9.9"));
        assert!(is_newer("v100.2235.626.2626", "100.2235.626.2625"));
    }

    #[test]
    fn normalize_strips_v() {
        assert_eq!(normalize("v0.0.0.1"), "0.0.0.1");
    }
}
