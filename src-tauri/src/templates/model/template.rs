use crate::core::script_field::ScriptField;
use crate::templates::trigger::{normalize_triggers, TimeTrigger};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;


use super::filters::{
    default_marketplace, default_report_period, default_tab_name, default_true,
    default_write_sort_direction, ExportSyncSettings, SubjectsListFilters,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub script_key: String,
    #[serde(default, alias = "SpreadsheetId", alias = "spreadsheetId")]
    pub spreadsheet_id: String,
    #[serde(default, alias = "SheetName", alias = "sheetName")]
    pub sheet_name: String,
    /// `ExportToExcel`: писать результат в `.xlsx` вместо Google Sheets.
    #[serde(default, alias = "ExportToExcel", alias = "exportToExcel")]
    pub export_to_excel: bool,
    /// Заголовок документа Google Sheets (кэш для карточек / редактора).
    #[serde(default, alias = "SpreadsheetTitle", alias = "spreadsheetTitle")]
    pub spreadsheet_title: String,
    /// Источник для скриптов вроде `wb_stocks` (Массив → nmId/chrtId).
    #[serde(default, alias = "SecondSpreadsheetId", alias = "secondSpreadsheetId")]
    pub second_spreadsheet_id: String,
    #[serde(default, alias = "SecondSheetName", alias = "secondSheetName")]
    pub second_sheet_name: String,
    #[serde(default, alias = "SecondSpreadsheetTitle", alias = "secondSpreadsheetTitle")]
    pub second_spreadsheet_title: String,
    /// Ozon: `sku` | `offer_id` | `product_id`.
    #[serde(default)]
    pub id_mode: String,
    /// Устаревший alias: синхронизируется с offer/product/sku range по `id_mode`.
    #[serde(default)]
    pub id_source_range: String,
    /// Унифицированные диапазоны (`CabinetsSourceRange` / …).
    #[serde(default, alias = "CabinetsSourceRange", alias = "cabinetsSourceRange")]
    pub cabinets_source_range: String,
    #[serde(default, alias = "NmIdSourceRange", alias = "nmIdSourceRange")]
    pub nm_id_source_range: String,
    /// chrtId для `wb_stocks` ( `StocksSkusSourceRange`).
    #[serde(default, alias = "StocksSkusSourceRange", alias = "stocksSkusSourceRange")]
    pub stocks_skus_source_range: String,
    /// ID РК для `wb_adv_campaigns_stats_new` ( `IdAdvCampaignSourceRange`).
    #[serde(
        default,
        alias = "IdAdvCampaignSourceRange",
        alias = "idAdvCampaignSourceRange"
    )]
    pub id_adv_campaign_source_range: String,
    #[serde(default, alias = "OfferIdSourceRange", alias = "offerIdSourceRange")]
    pub offer_id_source_range: String,
    #[serde(default, alias = "ProductIdSourceRange", alias = "productIdSourceRange")]
    pub product_id_source_range: String,
    #[serde(default, alias = "SkuSourceRange", alias = "skuSourceRange")]
    pub sku_source_range: String,
    /// Ссылки на медиа (`content_media_save`).
    #[serde(
        default,
        alias = "ContentMediaLinksSourceRange",
        alias = "contentMediaLinksSourceRange"
    )]
    pub content_media_links_source_range: String,
    /// ID предметов / категорий (`wb_tnved_codes`, `wb_object_charcs`).
    #[serde(default, alias = "SubjectIdSourceRange", alias = "subjectIdSourceRange")]
    pub subject_id_source_range: String,
    #[serde(
        default,
        alias = "SubjectNameSourceRange",
        alias = "subjectNameSourceRange"
    )]
    pub subject_name_source_range: String,
    #[serde(
        default,
        alias = "UseCustomSubjectIdsForTnved",
        alias = "useCustomSubjectIdsForTnved"
    )]
    pub use_custom_subject_ids: bool,
    /// Режимы `cards_update_dimensions`.
    #[serde(
        default = "default_true",
        alias = "CardsUpdateDimensionsMode",
        alias = "cardsUpdateDimensionsMode"
    )]
    pub cards_update_dimensions_mode: bool,
    #[serde(
        default,
        alias = "CardsUpdateVendorCodeMode",
        alias = "cardsUpdateVendorCodeMode"
    )]
    pub cards_update_vendor_code_mode: bool,
    #[serde(default, alias = "CardsUpdateVatMode", alias = "cardsUpdateVatMode")]
    pub cards_update_vat_mode: bool,
    #[serde(default, alias = "CardsUpdateBrandMode", alias = "cardsUpdateBrandMode")]
    pub cards_update_brand_mode: bool,
    #[serde(
        default,
        alias = "CardsUpdateLengthRange",
        alias = "cardsUpdateLengthRange"
    )]
    pub cards_update_length_range: String,
    #[serde(
        default,
        alias = "CardsUpdateWidthRange",
        alias = "cardsUpdateWidthRange"
    )]
    pub cards_update_width_range: String,
    #[serde(
        default,
        alias = "CardsUpdateHeightRange",
        alias = "cardsUpdateHeightRange"
    )]
    pub cards_update_height_range: String,
    #[serde(
        default,
        alias = "CardsUpdateVendorCodeRange",
        alias = "cardsUpdateVendorCodeRange"
    )]
    pub cards_update_vendor_code_range: String,
    #[serde(default, alias = "CardsUpdateVatRange", alias = "cardsUpdateVatRange")]
    pub cards_update_vat_range: String,
    #[serde(
        default,
        alias = "CardsUpdateBrandRange",
        alias = "cardsUpdateBrandRange"
    )]
    pub cards_update_brand_range: String,
    /// `prices_upload`: цена / скидка / exclude / raise %.
    #[serde(default, alias = "PricesPriceRange", alias = "pricesPriceRange")]
    pub prices_price_range: String,
    #[serde(
        default,
        alias = "PricesDiscountRange",
        alias = "pricesDiscountRange"
    )]
    pub prices_discount_range: String,
    #[serde(
        default,
        alias = "PricesExcludeRange",
        alias = "pricesExcludeRange"
    )]
    pub prices_exclude_range: String,
    #[serde(
        default,
        alias = "PricesRaisePercent",
        alias = "pricesRaisePercent"
    )]
    pub prices_raise_percent: String,
    /// `prices_set`.
    #[serde(
        default,
        alias = "PriceSetPriceRange",
        alias = "priceSetPriceRange"
    )]
    pub price_set_price_range: String,
    #[serde(
        default,
        alias = "PriceSetDiscountRange",
        alias = "priceSetDiscountRange"
    )]
    pub price_set_discount_range: String,
    /// `prices_history_task`.
    #[serde(
        default,
        alias = "PricesHistoryTaskUploadIdRange",
        alias = "pricesHistoryTaskUploadIdRange"
    )]
    pub prices_history_task_upload_id_range: String,
    /// `paid_storage_status` / download.
    #[serde(
        default,
        alias = "PaidStorageStatusTaskIdRange",
        alias = "paidStorageStatusTaskIdRange"
    )]
    pub paid_storage_status_task_id_range: String,
    #[serde(
        default,
        alias = "PaidStorageStatusOutputRange",
        alias = "paidStorageStatusOutputRange"
    )]
    pub paid_storage_status_output_range: String,
    #[serde(
        default,
        alias = "PaidStorageDownloadTaskIdRange",
        alias = "paidStorageDownloadTaskIdRange"
    )]
    pub paid_storage_download_task_id_range: String,
    /// `symbols_check` — A1-диапазон окраски.
    #[serde(
        default,
        alias = "SymbolsCheckRange",
        alias = "symbolsCheckRange"
    )]
    pub symbols_check_range: String,
    /// `cells_count` — считать заполненные ячейки.
    #[serde(
        default,
        alias = "CellsCountCountFilledCells",
        alias = "cellsCountCountFilledCells"
    )]
    pub cells_count_count_filled: bool,
    /// `promotions`: все акции, не только доступные для участия (`allPromo`).
    #[serde(default, alias = "PromotionsAllPromo", alias = "promotionsAllPromo")]
    pub promotions_all_promo: bool,
    /// `report_realization_v5` / `sales_reports_list`: `daily` | `weekly`.
    #[serde(
        default = "default_report_period",
        alias = "ReportRealizationV5Period",
        alias = "reportRealizationV5Period"
    )]
    pub report_realization_v5_period: String,
    /// Фильтры `subjects_list` ( `SubjectsListSettings`).
    #[serde(default, alias = "SubjectsFilters", alias = "subjectsFilters")]
    pub subjects_filters: SubjectsListFilters,
    #[serde(default)]
    pub date_from: String,
    #[serde(default)]
    pub date_to: String,
    /// Пресет дат : `1week` / `2weeks` / `4week` / …
    #[serde(default)]
    pub date_template_type: String,
    #[serde(default)]
    pub cabinets: Vec<String>,
    #[serde(default)]
    pub selected_fields: Vec<ScriptField>,
    #[serde(
        default,
        alias = "SyncSettings",
        alias = "syncSettings"
    )]
    pub sync_settings: ExportSyncSettings,
    /// Сортировка перед записью ( `WriteSortFieldName`).
    #[serde(
        default,
        alias = "WriteSortFieldName",
        alias = "writeSortFieldName"
    )]
    pub write_sort_field_name: String,
    /// `asc` | `desc` ( `WriteSortDirection`).
    #[serde(
        default = "default_write_sort_direction",
        alias = "WriteSortDirection",
        alias = "writeSortDirection"
    )]
    pub write_sort_direction: String,
    /// true = clear только колонок данных (справа пользовательские формулы не трогаем).
    /// По умолчанию false — очищается весь лист.
    #[serde(
        default,
        alias = "ClearOnlyWrittenRange",
        alias = "clearOnlyWrittenRange"
    )]
    pub clear_only_written_range: bool,
    #[serde(default = "default_marketplace")]
    pub marketplace: String,
    #[serde(default = "default_tab_name")]
    pub tab_name: String,
    #[serde(default)]
    pub last_run_status: String,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default, alias = "TimeTriggers")]
    pub time_triggers: Vec<TimeTrigger>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// Карточка списка шаблонов — без fields/ranges/sync/triggers (полный объект → `templates_get`).
#[derive(Debug, Clone, Serialize)]
pub struct TemplateCard {
    pub id: String,
    pub name: String,
    pub script_key: String,
    pub spreadsheet_id: String,
    pub sheet_name: String,
    pub export_to_excel: bool,
    pub spreadsheet_title: String,
    pub date_from: String,
    pub date_to: String,
    pub date_template_type: String,
    pub cabinets: Vec<String>,
    pub marketplace: String,
    pub tab_name: String,
    pub last_run_status: String,
    pub last_run_at: String,
}

impl From<&Template> for TemplateCard {
    fn from(t: &Template) -> Self {
        Self {
            id: t.id.clone(),
            name: t.name.clone(),
            script_key: t.script_key.clone(),
            spreadsheet_id: t.spreadsheet_id.clone(),
            sheet_name: t.sheet_name.clone(),
            export_to_excel: t.export_to_excel,
            spreadsheet_title: t.spreadsheet_title.clone(),
            date_from: t.date_from.clone(),
            date_to: t.date_to.clone(),
            date_template_type: t.date_template_type.clone(),
            cabinets: t.cabinets.clone(),
            marketplace: t.marketplace.clone(),
            tab_name: t.tab_name.clone(),
            last_run_status: t.last_run_status.clone(),
            last_run_at: t.last_run_at.clone(),
        }
    }
}

impl Template {
    pub fn new_blank(
        script_key: &str,
        script_title: &str,
        fields: Vec<ScriptField>,
        marketplace: &str,
        tab_name: &str,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name: crate::core::i18n::tf(
                "templates.newTemplateName",
                &[("scriptTitle", script_title)],
            ),
            script_key: script_key.to_string(),
            spreadsheet_id: String::new(),
            sheet_name: String::new(),
            export_to_excel: false,
            spreadsheet_title: String::new(),
            second_spreadsheet_id: String::new(),
            second_sheet_name: String::new(),
            second_spreadsheet_title: String::new(),
            id_mode: String::new(),
            id_source_range: String::new(),
            cabinets_source_range: String::new(),
            nm_id_source_range: String::new(),
            stocks_skus_source_range: String::new(),
            id_adv_campaign_source_range: String::new(),
            offer_id_source_range: String::new(),
            product_id_source_range: String::new(),
            sku_source_range: String::new(),
            content_media_links_source_range: String::new(),
            subject_id_source_range: String::new(),
            subject_name_source_range: String::new(),
            use_custom_subject_ids: false,
            cards_update_dimensions_mode: true,
            cards_update_vendor_code_mode: false,
            cards_update_vat_mode: false,
            cards_update_brand_mode: false,
            cards_update_length_range: String::new(),
            cards_update_width_range: String::new(),
            cards_update_height_range: String::new(),
            cards_update_vendor_code_range: String::new(),
            cards_update_vat_range: String::new(),
            cards_update_brand_range: String::new(),
            prices_price_range: String::new(),
            prices_discount_range: String::new(),
            prices_exclude_range: String::new(),
            prices_raise_percent: String::new(),
            price_set_price_range: String::new(),
            price_set_discount_range: String::new(),
            prices_history_task_upload_id_range: String::new(),
            paid_storage_status_task_id_range: String::new(),
            paid_storage_status_output_range: String::new(),
            paid_storage_download_task_id_range: String::new(),
            symbols_check_range: String::new(),
            cells_count_count_filled: false,
            promotions_all_promo: false,
            report_realization_v5_period: default_report_period(),
            subjects_filters: SubjectsListFilters::default(),
            date_from: String::new(),
            date_to: String::new(),
            date_template_type: String::new(),
            cabinets: Vec::new(),
            selected_fields: fields,
            sync_settings: ExportSyncSettings::default(),
            write_sort_field_name: String::new(),
            write_sort_direction: default_write_sort_direction(),
            clear_only_written_range: false,
            marketplace: marketplace.to_string(),
            tab_name: if tab_name.trim().is_empty() {
                default_tab_name()
            } else {
                tab_name.to_string()
            },
            last_run_status: String::new(),
            last_run_at: String::new(),
            time_triggers: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn normalize(&mut self) {
        self.time_triggers = normalize_triggers(std::mem::take(&mut self.time_triggers));
        self.sync_ozon_id_ranges();
    }

    /// `id_source_range` ↔ offer/product/sku по `id_mode` (паритет + совместимость UR).
    pub fn sync_ozon_id_ranges(&mut self) {
        let mode = self.id_mode.trim().to_ascii_lowercase();
        let legacy = self.id_source_range.trim().to_string();
        let named = match mode.as_str() {
            "offer_id" => &mut self.offer_id_source_range,
            "product_id" => &mut self.product_id_source_range,
            _ => &mut self.sku_source_range,
        };
        if named.trim().is_empty() && !legacy.is_empty() {
            *named = legacy.clone();
        }
        if !named.trim().is_empty() {
            self.id_source_range = named.trim().to_string();
        }
    }
}

