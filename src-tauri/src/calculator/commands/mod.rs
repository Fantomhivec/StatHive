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

//! Tauri IPC для калькулятора.

mod lookup;
mod run;
mod settings;
mod snapshot;
mod team;

pub use lookup::*;
pub use run::*;
pub use settings::*;
pub use snapshot::*;
pub use team::*;

use crate::app_state::AppState;
use crate::core::access;
use crate::core::google_sheets::GoogleSheetsClient;
use crate::core::i18n::t;

/// Admin / участник с рабочим OAuth — user token; иначе SA (просмотр без своего Google).
pub(super) async fn calc_sheets(state: &AppState) -> Result<GoogleSheetsClient, String> {
    let use_user_oauth = state
        .auth
        .current_user()
        .map(|u| access::effective_has_oauth(&u.access_level, u.has_oauth))
        .unwrap_or(false);
    if use_user_oauth {
        GoogleSheetsClient::from_user_oauth().await
    } else {
        GoogleSheetsClient::from_service_account().await
    }
}

pub(super) fn require_admin(state: &AppState) -> Result<(), String> {
    let u = crate::templates::access_gate::require_user(state)?;
    if !access::can_manage_team(&u.access_level) {
        return Err(t("errors.auth.teamCodeAdminOnly"));
    }
    Ok(())
}
