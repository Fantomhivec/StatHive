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

//! Summary sheet maps (WB/OZ sources).
mod enrich;
mod sheet_map;
mod wb_load;

pub use enrich::{
    apply_commission_from_map, apply_logistics_from_box, cell, first_nonempty_line, photo_url, put,
};
pub(crate) use sheet_map::{
    load_sheet_map, load_table_rows,
};
pub use wb_load::load_wb_maps;
pub(crate) use wb_load::merge_sheet_cache;
