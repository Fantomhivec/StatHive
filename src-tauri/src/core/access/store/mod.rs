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

//! Preset sheet store.
mod flags;
mod presets_io;

pub use flags::{
    AccessMatrix, PresetDef, ADMIN_ROLE_ID, ALL_APP_FLAGS, ALL_CALC_FLAGS, APP_FLAG_ALL_TABLES,
    APP_FLAG_KEYS_CHECK, APP_FLAG_USERS, CALC_FLAG_EXCEL, CALC_FLAG_KGT, CALC_FLAG_SETTINGS,
    CALC_FLAG_TABLES, CALC_FLAG_VIEW,
};
pub use presets_io::{
    delete_preset, ensure_access_sheets, list_presets, reload_presets_cache, resolve_preset_id,
    save_preset,
};
