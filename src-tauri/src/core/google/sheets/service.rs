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

//! Unified table dump API: replace (clear scopes) + incremental sync.

use super::GoogleSheetsClient;

/// How wide to clear before a full replace (rows always = used-range, not grid.rowCount).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetClearScope {
    /// Template `clear_only_written_range`: A1:{dataCols}{usedRows}.
    DataColumns,
    /// Default template («весь лист» in UI): A1:{max(data, gridCols)}{usedRows}.
    DataAndGridWidth,
}

impl SheetClearScope {
    pub fn from_clear_only_written_range(clear_only: bool) -> Self {
        if clear_only {
            Self::DataColumns
        } else {
            Self::DataAndGridWidth
        }
    }

    pub fn log_label(self) -> &'static str {
        match self {
            Self::DataColumns => "только колонки данных",
            Self::DataAndGridWidth => "used-range + ширина сетки",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SheetWriteMode {
    Replace { clear: SheetClearScope },
    Incremental {
        key_columns: Vec<usize>,
        allow_delete: bool,
    },
}

pub struct SheetWriteRequest {
    pub spreadsheet_id: String,
    pub sheet_name: String,
    pub rows: Vec<Vec<String>>,
    pub mode: SheetWriteMode,
}

#[derive(Debug)]
pub enum SheetWriteOutcome {
    Replaced { written: usize },
    Incremental {
        updated_rows: usize,
        inserted_rows: usize,
        deleted_rows: usize,
    },
}

impl SheetWriteOutcome {
    pub fn rows_affected(&self) -> usize {
        match self {
            Self::Replaced { written } => *written,
            Self::Incremental {
                updated_rows,
                inserted_rows,
                ..
            } => updated_rows + inserted_rows,
        }
    }
}

impl GoogleSheetsClient {
    /// Single entry for table dumps (scripts + calculator). Cell patches stay on `write_values`.
    pub async fn write_sheet(
        &self,
        req: SheetWriteRequest,
        mut on_progress: impl FnMut(String),
    ) -> Result<SheetWriteOutcome, String> {
        let sid = req.spreadsheet_id.trim();
        let sheet = req.sheet_name.trim();
        if sid.is_empty() || sheet.is_empty() {
            return Err("Укажите spreadsheetId и имя листа".into());
        }
        match req.mode {
            SheetWriteMode::Replace { clear } => {
                let written = self
                    .clear_and_write(sid, sheet, req.rows, clear, &mut on_progress)
                    .await?;
                Ok(SheetWriteOutcome::Replaced { written })
            }
            SheetWriteMode::Incremental {
                key_columns,
                allow_delete,
            } => {
                let stats = self
                    .write_incremental(
                        sid,
                        sheet,
                        req.rows,
                        &key_columns,
                        allow_delete,
                        &mut on_progress,
                    )
                    .await?;
                Ok(SheetWriteOutcome::Incremental {
                    updated_rows: stats.updated_rows,
                    inserted_rows: stats.inserted_rows,
                    deleted_rows: stats.deleted_rows,
                })
            }
        }
    }
}
