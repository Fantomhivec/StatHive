//! Shared script helpers (cabinets, sheets write, sync).
mod resolve;
mod write;

pub use resolve::{
    append_cabinet_rows, empty_cabinets_output, empty_data_output, ensure_header_row, is_zero_numeric_id,
    load_ozon_pairs, load_wb_tokens, map_cabinets_chunked, ozon_pair_from_map, resolve_cabinets,
    resolve_fields, resolve_spreadsheet_target, resolve_wb_token_with_map, selected_titles,
    CABINET_PARALLELISM, OzonPairMap, WbTokenMap,
};
pub use write::{google_sheets_client, write_sheet_rows, write_sheet_rows_with};
