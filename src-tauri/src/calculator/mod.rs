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

mod access_guard;
pub mod article_index;
pub mod card_reader;
mod card_reader_fill;
mod card_summary_row;
pub mod catalog;
mod catalog_cols;
mod catalog_metrics;
pub mod commands;
mod config_io;
pub mod excel_export;
mod export_advertising;
mod export_service;
mod manual_bindings;
pub mod logistics;
pub mod profit;
pub mod profile;
pub mod run_log;
mod script_extras;
pub mod sheet_util;
pub mod snapshot;
mod snapshot_mut;
pub(crate) mod triggers;
mod summary_builder;
mod summary_maps;
mod summary_oz;
mod summary_oz_row;
mod summary_oz_orders;
pub mod team_config;
mod team_sync;
pub use team_sync::{set_app_handle as set_team_sync_app_handle, sync_assigned_config};

pub use commands::*;
