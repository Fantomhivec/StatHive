//! Preset sheet store.
mod flags;
mod presets_io;

pub use flags::{
    AccessMatrix, PresetDef, ADMIN_ROLE_ID, ALL_APP_FLAGS, ALL_CALC_FLAGS, APP_FLAG_KEYS_CHECK,
    APP_FLAG_USERS, CALC_FLAG_EXCEL, CALC_FLAG_KGT, CALC_FLAG_SETTINGS, CALC_FLAG_TABLES,
    CALC_FLAG_VIEW,
};
pub use presets_io::{
    delete_preset, ensure_access_sheets, list_presets, reload_presets_cache, resolve_preset_id,
    save_preset,
};
