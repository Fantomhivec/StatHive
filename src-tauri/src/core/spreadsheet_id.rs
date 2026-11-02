// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
