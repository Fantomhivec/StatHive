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

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(super) const INDEX_NAME: &str = "teams_index.json";
pub(super) const TEAMS_DIR: &str = "teams";
pub(super) const LEGACY_FILES: &[&str] = &[
    "workspace.json",
    "oauth_client.json",
    "service_account.json",
    "google_oauth_token.json",
    "session.json",
    "wb_calculator_settings.json",
    "calc_team_applied.json",
    "templates.json",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamIndexEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub joined_by_code: bool,
    /// `Admin` | `Member` — после успешного login.
    #[serde(default)]
    pub last_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamsIndex {
    #[serde(default)]
    pub active_id: String,
    #[serde(default)]
    pub teams: Vec<TeamIndexEntry>,
}

pub fn team_id_from_spreadsheet(spreadsheet_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(spreadsheet_id.trim().as_bytes());
    let dig = h.finalize();
    hex::encode(&dig[..8])
}

/// Цель — Admin (для switch: браузер Google).
pub fn target_needs_google(entry: &TeamIndexEntry) -> bool {
    let role = entry.last_role.trim();
    if role.eq_ignore_ascii_case("Admin") {
        return true;
    }
    if role.eq_ignore_ascii_case("Member") {
        return false;
    }
    // Нет lastRole: локально созданная команда → Admin.
    !entry.joined_by_code
}
