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

//! Runtime скриптов шаблонов/калькулятора: контекст, поля, лог.

mod context;

use serde::{Deserialize, Serialize};

pub use context::{
    format_label, MemberOk, Script, ScriptContext, ScriptOutput, ScriptRegistry, ScriptResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptField {
    pub name: String,
    pub title: String,
    #[serde(default = "default_true")]
    pub selected: bool,
}

fn default_true() -> bool {
    true
}

impl ScriptField {
    pub fn new(name: &str, title: impl Into<String>, selected: bool) -> Self {
        Self {
            name: name.to_string(),
            title: title.into(),
            selected,
        }
    }
}
