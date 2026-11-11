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

//! Persist templates.json — cache, CRUD, tabs, groups.

mod groups;
mod io;
mod tabs;
mod templates;

pub use groups::{
    add_to_group, delete_group, get_group, selected_tab_for, set_group_expanded, set_group_last_run,
    upsert_group,
};
pub use io::invalidate_cache;
pub(crate) use io::{with_file, with_file_mut};
pub use tabs::{add_tab, delete_tab, rename_tab, select_tab, set_tab_color};
pub use templates::{
    backfill_spreadsheet_target, backfill_spreadsheet_targets, clear_stuck_excel_exports,
    clear_stuck_excel_exports_once, delete, get, scrub_stuck_running_last_runs_once, set_last_run,
    snapshot, upsert,
};
pub(crate) use templates::find_market_for_template;
