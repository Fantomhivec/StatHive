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

//! WB supplies cluster: FBS list / FBO list / info / goods / warehouses.

pub mod api;
pub mod fbo;
pub mod goods;
pub mod info;
pub mod list;
pub mod warehouses;

pub use fbo::SuppliesFboScript;
pub use goods::SuppliesGoodsScript;
pub use info::SupplyInfoScript;
pub use list::SuppliesScript;
pub use warehouses::SuppliesWarehousesScript;
