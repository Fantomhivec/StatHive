use chrono::{DateTime, NaiveDate, NaiveDateTime, Timelike};

pub const DATE_FORMAT: &str = "%d.%m.%Y";
pub const DATETIME_FORMAT: &str = "%d.%m.%Y %H:%M:%S";

pub fn format_date(d: NaiveDate) -> String {
    d.format(DATE_FORMAT).to_string()
}

pub fn format_datetime(dt: NaiveDateTime) -> String {
    if dt.time().num_seconds_from_midnight() == 0 {
        format_date(dt.date())
    } else {
        dt.format(DATETIME_FORMAT).to_string()
    }
}

/// WB ISO / RFC3339 → `dd.MM.yyyy` or `dd.MM.yyyy HH:mm:ss` for sheet/CSV cells.
pub fn format_wb_timestamp(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if let Some(canon) = try_normalize_date_for_compare(s) {
        return canon;
    }
    s.to_string()
}

/// Парсинг IncrementalSheetWriter.TryNormalizeDate → SheetDateFormatter.Format.
/// Kind/TZ игнорируются (визуальное сравнение).
pub fn try_normalize_date_for_compare(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(format_datetime(dt.naive_local()));
    }

    // Точные форматы (Invariant / без TZ).
    const EXACT: &[&str] = &[
        "%d.%m.%Y",
        "%d.%m.%Y %H:%M:%S",
        "%d/%m/%Y",
        "%d/%m/%Y %H:%M:%S",
        "%Y-%m-%d",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ];
    for fmt in EXACT {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(format_datetime(dt));
        }
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(format_date(d));
        }
    }

    // RFC3339-подобные с offset без полной поддержки chrono: …+03:00 / Z уже выше.
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z") {
        return Some(format_datetime(dt.naive_local()));
    }
    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z") {
        return Some(format_datetime(dt.naive_local()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_iso_equals_dot() {
        assert_eq!(
            try_normalize_date_for_compare("2026-10-27").as_deref(),
            Some("27.10.2026")
        );
        assert_eq!(
            try_normalize_date_for_compare("27.10.2026").as_deref(),
            Some("27.10.2026")
        );
        assert_eq!(
            try_normalize_date_for_compare("27/10/2026").as_deref(),
            Some("27.10.2026")
        );
    }

    #[test]
    fn datetime_iso_equals_dot() {
        assert_eq!(
            try_normalize_date_for_compare("2026-10-27T14:30:00").as_deref(),
            Some("27.10.2026 14:30:00")
        );
        assert_eq!(
            try_normalize_date_for_compare("27.10.2026 14:30:00").as_deref(),
            Some("27.10.2026 14:30:00")
        );
    }

    #[test]
    fn midnight_strips_time() {
        assert_eq!(
            try_normalize_date_for_compare("2026-10-27T00:00:00").as_deref(),
            Some("27.10.2026")
        );
    }

    #[test]
    fn not_a_date() {
        assert!(try_normalize_date_for_compare("100").is_none());
        assert!(try_normalize_date_for_compare("abc").is_none());
    }
}
