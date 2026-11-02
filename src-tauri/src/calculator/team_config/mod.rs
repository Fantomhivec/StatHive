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

//! Team calculator configs (CalcTeam sheet + Drive share).
mod catalog;
mod drive;
mod types;

pub use catalog::{
    apply_entry_locally, delete_config, find_by_id, find_by_name, list_items, load_catalog,
    publish, pull_by_id, resolve_entry, set_default,
};
pub use drive::{keep_going, needs_apply, share_tables_with_service_account, sync_drive_access};
pub use types::{
    load_stamp, profile_for_user_drive, table_ids_from_profile, TeamConfigEntry,
};
