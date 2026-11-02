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

//! Tauri commands (тонкий facade; реализация в submodule).

mod access_cmds;
mod auth_cmds;
mod env_cmds;
mod i18n_cmds;
mod scripts_cmds;
mod updates_cmds;

pub use access_cmds::*;
pub use auth_cmds::*;
pub use env_cmds::*;
pub use i18n_cmds::*;
pub use scripts_cmds::*;
pub use updates_cmds::*;
