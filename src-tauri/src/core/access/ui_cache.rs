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

//! Кэш UI вкладки «Пользователи»: пресеты + команда.
//! Инвалидация: SA, invite, save/delete preset, назначение пресета.

use super::clear_presets_cache_inner;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamCacheRow {
    pub login: String,
    pub display_name: String,
    pub access_level: String,
    pub google_email: String,
    pub row_index: u32,
    #[serde(default)]
    pub calc_config_id: String,
}

static TEAM_CACHE: Mutex<Option<Vec<TeamCacheRow>>> = Mutex::new(None);

pub fn team_cache_ready() -> bool {
    TEAM_CACHE
        .lock()
        .ok()
        .map(|g| g.is_some())
        .unwrap_or(false)
}

pub fn cached_team() -> Option<Vec<TeamCacheRow>> {
    TEAM_CACHE.lock().ok().and_then(|g| g.clone())
}

pub fn set_team_cache(rows: Vec<TeamCacheRow>) {
    if let Ok(mut g) = TEAM_CACHE.lock() {
        *g = Some(rows);
    }
}

/// Готовность снимка вкладки «Пользователи».
/// Достаточно кэша команды — повторный заход не трогает Sheets/SA (пока force / invalidate).
pub fn users_ui_cache_ready() -> bool {
    team_cache_ready()
}

pub fn invalidate_team_cache() {
    if let Ok(mut g) = TEAM_CACHE.lock() {
        *g = None;
    }
}

/// Полный сброс: SA / смена источника данных.
pub fn invalidate_users_ui_cache() {
    clear_presets_cache_inner();
    invalidate_team_cache();
}
