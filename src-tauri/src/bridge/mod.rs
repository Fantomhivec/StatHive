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
