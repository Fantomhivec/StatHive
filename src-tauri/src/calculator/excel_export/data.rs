//! Сборка листов «Данные ВБ» / «Данные OZON» из сводов (документы уже в своде).

use crate::calculator::catalog::{self, OZ_SUMMARY_KEY, WB_SUMMARY_KEY};
use crate::core::i18n::t;
use crate::calculator::profile::CalculatorProfile;
use crate::calculator::sheet_util::{
    self, cell_at, find_header_index, get_sheet_rows, header_row, is_valid_article, ARTICLE_ALIASES,
    SheetCache,
};
use crate::core::google_sheets::GoogleSheetsClient;
use std::collections::HashMap;

pub struct DataTable {
    pub headers: Vec<String>,
    pub rows: Vec<HashMap<String, String>>,
}

impl DataTable {
    fn new_with_headers(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    pub(crate) fn ensure_header(&mut self, h: &str) {
        if self
            .headers
            .iter()
            .any(|x| x.eq_ignore_ascii_case(h))
        {
            return;
        }
        self.headers.push(h.to_string());
    }
}

pub async fn build_wb_table(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    warnings: &mut Vec<String>,
) -> DataTable {
    // WB_HEADERS включает документы / ТН-ВЭД — только свод, без добора с Массива.
    let mut table = DataTable::new_with_headers(catalog::wb_headers());

    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(WB_SUMMARY_KEY))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        warnings.push("Карточка WB: нет ссылки — лист «Данные ВБ» пустой.".into());
        return table;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        warnings.push("Карточка WB: не удалось разобрать ссылку.".into());
        return table;
    };
    let sheet = catalog::find_table(WB_SUMMARY_KEY)
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| t("calc.tables.WbSummary.sheetName"));
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
        warnings.push("Карточка WB: лист пуст или недоступен. Сначала обновите таблицы.".into());
        return table;
    };
    if rows.len() < 2 {
        warnings.push("Карточка WB: лист пуст. Сначала обновите таблицы.".into());
        return table;
    }

    let header = header_row(&rows);
    let art_label = t("calc.headers.wb.article");
    let art_idx = find_header_index(&header, &art_label, ARTICLE_ALIASES);
    let art_idx = if art_idx < 0 { 0 } else { art_idx };

    for row in rows.iter().skip(1) {
        let raw_art = cell_at(row, art_idx);
        if !is_valid_article(&raw_art) {
            continue;
        }
        let mut dict = HashMap::new();
        for (c, h) in header.iter().enumerate() {
            if h.trim().is_empty() {
                continue;
            }
            table.ensure_header(h);
            let v = cell_at(row, c as isize);
            if !v.is_empty() {
                dict.insert(h.clone(), v);
            }
        }
        dict.insert(art_label.clone(), raw_art.trim().to_string());
        table.rows.push(dict);
    }

    super::overlay::overlay_manual_bindings(profile, gs, cache, &mut table, false, warnings).await;
    table
}

/// Лист «Данные OZON» из свода OzSummary (паритет BuildMarketplaceTable isOzon:true).
pub async fn build_oz_table(
    profile: &CalculatorProfile,
    gs: &GoogleSheetsClient,
    cache: &mut SheetCache,
    warnings: &mut Vec<String>,
) -> DataTable {
    let mut table = DataTable::new_with_headers(catalog::oz_headers());

    let Some(url) = profile
        .table_links
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(OZ_SUMMARY_KEY))
        .map(|(_, v)| v.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        warnings.push("Карточка Ozon: нет ссылки — лист «Данные OZON» пустой (заголовки готовы).".into());
        return table;
    };
    let Some(sid) = sheet_util::extract_spreadsheet_id(&url) else {
        warnings.push("Карточка Ozon: не удалось разобрать ссылку.".into());
        return table;
    };
    let sheet = catalog::find_table(OZ_SUMMARY_KEY)
        .map(|t| t.sheet_name_tr())
        .unwrap_or_else(|| t("calc.tables.OzSummary.sheetName"));
    let Some(rows) = get_sheet_rows(gs, cache, &sid, &sheet).await else {
        warnings.push("Карточка Ozon: лист пуст или недоступен. Сначала обновите таблицы.".into());
        return table;
    };
    if rows.len() < 2 {
        warnings.push("Карточка Ozon: лист пуст. Сначала обновите таблицы.".into());
        return table;
    }

    let header = header_row(&rows);
    let sku_label = t("calc.headers.oz.sku");
    let art_idx = find_header_index(&header, &sku_label, ARTICLE_ALIASES);
    let art_idx = if art_idx < 0 { 0 } else { art_idx };

    for row in rows.iter().skip(1) {
        let raw_art = cell_at(row, art_idx);
        if !is_valid_article(&raw_art) {
            continue;
        }
        let mut dict = HashMap::new();
        for (c, h) in header.iter().enumerate() {
            if h.trim().is_empty() {
                continue;
            }
            table.ensure_header(h);
            let v = cell_at(row, c as isize);
            if !v.is_empty() {
                dict.insert(h.clone(), v);
            }
        }
        dict.insert(sku_label.clone(), raw_art.trim().to_string());
        table.rows.push(dict);
    }
    super::overlay::overlay_manual_bindings(profile, gs, cache, &mut table, true, warnings).await;
    table
}
