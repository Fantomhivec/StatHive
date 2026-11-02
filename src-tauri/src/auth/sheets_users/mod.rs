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
