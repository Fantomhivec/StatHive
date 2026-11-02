//! Локаль UI (Ru).

use crate::core::i18n;

#[tauri::command]
pub fn i18n_get_locale() -> String {
    i18n::locale()
}

#[tauri::command]
pub fn i18n_set_locale(locale: String) -> Result<String, String> {
    i18n::set_locale(&locale)?;
    Ok(i18n::locale())
}
