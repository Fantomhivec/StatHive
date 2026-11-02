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

//! Профили лимитов для скриптов UR registry (паритет RateLimiterProfiles).

mod content;
mod wb_misc;
mod wb_reports;
pub mod ozon;

pub use content::*;
pub use wb_misc::*;
pub use wb_reports::*;
