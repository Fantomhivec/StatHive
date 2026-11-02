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
