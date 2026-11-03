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

use std::collections::HashMap;

/// Собирает params для ScriptContext из полей шаблона.
pub(super) fn build_run_params(
    tpl: &crate::templates::Template,
    resolved_sid: &str,
    resolved_sheet: &str,
    export_to_excel: bool,
    excel_file_path: Option<&str>,
    dry_run: bool,
) -> HashMap<String, String> {
    let mut params = HashMap::new();
    params.insert("spreadsheet_id".into(), resolved_sid.to_string());
    params.insert(
        "sheet_name".into(),
        if export_to_excel && resolved_sheet.trim().is_empty() {
            "Sheet1".into()
        } else {
            resolved_sheet.to_string()
        },
    );
    if export_to_excel {
        params.insert("export_to_excel".into(), "true".into());
        if let Some(path) = excel_file_path {
            params.insert("excel_file_path".into(), path.to_string());
        }
    }
    if !tpl.second_spreadsheet_id.trim().is_empty() {
        let sid = tpl.second_spreadsheet_id.clone();
        params.insert("source_spreadsheet_id".into(), sid.clone());
        params.insert("second_spreadsheet_id".into(), sid);
    }
    if !tpl.second_sheet_name.trim().is_empty() {
        let sname = tpl.second_sheet_name.clone();
        params.insert("source_sheet_name".into(), sname.clone());
        params.insert("second_sheet_name".into(), sname);
    }
    if !tpl.id_mode.trim().is_empty() {
        params.insert("id_mode".into(), tpl.id_mode.clone());
    }
    for (key, val) in [
        ("cabinets_source_range", &tpl.cabinets_source_range),
        ("nm_id_source_range", &tpl.nm_id_source_range),
        ("stocks_skus_source_range", &tpl.stocks_skus_source_range),
        (
            "id_adv_campaign_source_range",
            &tpl.id_adv_campaign_source_range,
        ),
        ("offer_id_source_range", &tpl.offer_id_source_range),
        ("product_id_source_range", &tpl.product_id_source_range),
        ("sku_source_range", &tpl.sku_source_range),
        (
            "content_media_links_source_range",
            &tpl.content_media_links_source_range,
        ),
        ("subject_id_source_range", &tpl.subject_id_source_range),
        ("subject_name_source_range", &tpl.subject_name_source_range),
        ("cards_update_length_range", &tpl.cards_update_length_range),
        ("cards_update_width_range", &tpl.cards_update_width_range),
        ("cards_update_height_range", &tpl.cards_update_height_range),
        (
            "cards_update_vendor_code_range",
            &tpl.cards_update_vendor_code_range,
        ),
        ("cards_update_vat_range", &tpl.cards_update_vat_range),
        ("cards_update_brand_range", &tpl.cards_update_brand_range),
        ("prices_price_range", &tpl.prices_price_range),
        ("prices_discount_range", &tpl.prices_discount_range),
        ("prices_exclude_range", &tpl.prices_exclude_range),
        ("prices_raise_percent", &tpl.prices_raise_percent),
        ("price_set_price_range", &tpl.price_set_price_range),
        ("price_set_discount_range", &tpl.price_set_discount_range),
        (
            "prices_history_task_upload_id_range",
            &tpl.prices_history_task_upload_id_range,
        ),
        (
            "paid_storage_status_task_id_range",
            &tpl.paid_storage_status_task_id_range,
        ),
        (
            "paid_storage_status_output_range",
            &tpl.paid_storage_status_output_range,
        ),
        (
            "paid_storage_download_task_id_range",
            &tpl.paid_storage_download_task_id_range,
        ),
        ("symbols_check_range", &tpl.symbols_check_range),
        (
            "report_realization_v5_period",
            &tpl.report_realization_v5_period,
        ),
    ] {
        if !val.trim().is_empty() {
            params.insert(key.into(), val.clone());
        }
    }
    if tpl.use_custom_subject_ids {
        params.insert("use_custom_subject_ids".into(), "true".into());
    }
    if tpl.cells_count_count_filled {
        params.insert("cells_count_count_filled".into(), "true".into());
    }
    if tpl.promotions_all_promo {
        params.insert("promotions_all_promo".into(), "true".into());
    }
    params.insert(
        "cards_update_dimensions_mode".into(),
        if tpl.cards_update_dimensions_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_vendor_code_mode".into(),
        if tpl.cards_update_vendor_code_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_vat_mode".into(),
        if tpl.cards_update_vat_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    params.insert(
        "cards_update_brand_mode".into(),
        if tpl.cards_update_brand_mode {
            "true".into()
        } else {
            "false".into()
        },
    );
    if let Ok(json) = serde_json::to_string(&tpl.subjects_filters) {
        params.insert("subjects_filters_json".into(), json);
    }
    // Sync Ozon id ranges without cloning the whole Template (selected_fields / triggers).
    let mode = tpl.id_mode.trim().to_ascii_lowercase();
    let legacy = tpl.id_source_range.trim();
    let mut sku_range = tpl.sku_source_range.clone();
    let mut offer_range = tpl.offer_id_source_range.clone();
    let mut product_range = tpl.product_id_source_range.clone();
    {
        let named = match mode.as_str() {
            "offer_id" => &mut offer_range,
            "product_id" => &mut product_range,
            _ => &mut sku_range,
        };
        if named.trim().is_empty() && !legacy.is_empty() {
            *named = legacy.to_string();
        }
    }
    let active = match mode.as_str() {
        "offer_id" => offer_range.trim(),
        "product_id" => product_range.trim(),
        _ => sku_range.trim(),
    };
    if !active.is_empty() {
        params.insert("id_source_range".into(), active.to_string());
        params.insert("sku_source_range".into(), active.to_string());
        match mode.as_str() {
            "offer_id" => {
                params.insert("offer_id_source_range".into(), active.to_string());
            }
            "product_id" => {
                params.insert("product_id_source_range".into(), active.to_string());
            }
            _ => {}
        }
    }
    if !tpl.date_from.trim().is_empty() {
        params.insert("date_from".into(), tpl.date_from.clone());
    }
    if !tpl.date_to.trim().is_empty() {
        params.insert("date_to".into(), tpl.date_to.clone());
    }
    if !tpl.cabinets.is_empty() {
        params.insert("cabinets".into(), tpl.cabinets.join(","));
        params.insert("cabinet".into(), tpl.cabinets[0].clone());
    }
    if let Ok(json) = serde_json::to_string(&tpl.selected_fields) {
        params.insert("selected_fields_json".into(), json);
    }
    if let Ok(json) = serde_json::to_string(&tpl.sync_settings) {
        params.insert("sync_settings_json".into(), json);
    }
    if !tpl.write_sort_field_name.trim().is_empty() {
        params.insert(
            "write_sort_field_name".into(),
            tpl.write_sort_field_name.trim().to_string(),
        );
        let dir = tpl.write_sort_direction.trim().to_ascii_lowercase();
        params.insert(
            "write_sort_direction".into(),
            if dir == "desc" {
                "desc".into()
            } else {
                "asc".into()
            },
        );
    }
    params.insert(
        "clear_only_written_range".into(),
        if tpl.clear_only_written_range {
            "true".into()
        } else {
            "false".into()
        },
    );
    if dry_run {
        params.insert("dry_run".into(), "true".into());
    }
    params
}
