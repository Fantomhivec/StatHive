//! Ядро приложения. Кластеры ≥3 файлов — во вложенных папках (`env`, `google`, `http`, `script`, `excel`).
//! Одиночки и пары остаются в корне. Совместимые `pub use` — старые пути `crate::core::…`.

pub mod access;
pub mod app_version;
pub mod config;
pub mod env;
pub mod excel;
pub mod google;
pub mod http;
pub mod i18n;
pub mod log;
pub mod oauth_config;
pub mod password_hash;
pub mod paths;
pub mod secure_store;
pub mod service_account_cfg;
pub mod script;
pub mod sheet_dates;
pub mod spreadsheet_id;
pub mod tabs_layout;
pub mod team_invite;
pub mod team_profile;
pub mod workspace_cfg;

// --- совместимые алиасы (старый flat layout) ---
pub use env::check as env_check;
pub use env::keys as env_keys;
pub use env::wb_token;

pub use google::oauth as google_oauth;
pub use google::sheets as google_sheets;

pub use http::rate_limit;
pub use http::rate_profiles;
pub use http::request_sender;

/// Совместимый путь `crate::core::script_field::ScriptField`.
pub mod script_field {
    pub use crate::core::script::ScriptField;
}
