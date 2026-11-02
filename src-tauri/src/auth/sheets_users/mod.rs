//! Лист «Пользователи» в workspace spreadsheet.
//! Чтение/запись — через Service Account (если настроен), иначе user OAuth (Admin-владелец).

mod model;
mod read;
mod write;

pub use model::{has_active_foreign_session, SheetUser, USERS_SHEET_NAME};
pub use read::{
    auth_client, find_user_by_credentials, find_user_by_credentials_scan, list_users,
    read_user_calc_config, read_user_row, spreadsheet_id, user_client,
};
pub use write::{
    append_user_row, clear_session_cells, delete_user_row, write_access_level, write_activity,
    write_calc_config, write_google_email, write_has_oauth, write_header, write_last_login,
    write_password, write_session,
};
