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

//! WB warehouse remains: create / status / download / auto.

pub mod api;
pub mod auto;
pub mod create;
pub mod download;
pub mod status;

pub use auto::WarehouseRemainsAutoScript;
pub use create::WarehouseRemainsCreateTaskScript;
pub use download::WarehouseRemainsDownloadScript;
pub use status::WarehouseRemainsStatusScript;
