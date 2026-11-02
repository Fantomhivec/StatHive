//! Каталог таблиц и метрик WB + Ozon.

use crate::core::i18n::t;

#[derive(Debug, Clone, Copy)]
pub struct TableDef {
    pub key: &'static str,
    pub label: &'static str,
    pub spreadsheet_title: &'static str,
    pub sheet_name: &'static str,
    pub script_key: &'static str,
    pub marketplace: &'static str,
}

impl TableDef {
    pub fn label_tr(&self) -> String {
        t(self.label)
    }

    pub fn spreadsheet_title_tr(&self) -> String {
        t(self.spreadsheet_title)
    }

    pub fn sheet_name_tr(&self) -> String {
        t(self.sheet_name)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetricDef {
    pub key: &'static str,
    pub label: &'static str,
    pub marketplace: &'static str,
    pub default_table_key: &'static str,
    pub default_sheet_name: &'static str,
    pub default_column_header: &'static str,
    pub default_article_header: &'static str,
}

impl MetricDef {
    pub fn label_tr(&self) -> String {
        t(self.label)
    }

    pub fn default_sheet_name_tr(&self) -> String {
        t(self.default_sheet_name)
    }

    pub fn default_column_header_tr(&self) -> String {
        t(self.default_column_header)
    }

    pub fn default_article_header_tr(&self) -> String {
        t(self.default_article_header)
    }
}

pub const WB_SUMMARY_KEY: &str = "WbSummary";
pub const OZ_SUMMARY_KEY: &str = "OzSummary";

pub const WB_TABLES: &[TableDef] = &[
    TableDef {
        key: "Array",
        label: "calc.tables.Array.label",
        spreadsheet_title: "calc.tables.Array.spreadsheetTitle",
        sheet_name: "calc.tables.Array.sheetName",
        script_key: "cards_list",
        marketplace: "wb",
    },
    TableDef {
        key: "Commissions",
        label: "calc.tables.Commissions.label",
        spreadsheet_title: "calc.tables.Commissions.spreadsheetTitle",
        sheet_name: "calc.tables.Commissions.sheetName",
        script_key: "tariffs_commission",
        marketplace: "wb",
    },
    TableDef {
        key: "BoxTariffs",
        label: "calc.tables.BoxTariffs.label",
        spreadsheet_title: "calc.tables.BoxTariffs.spreadsheetTitle",
        sheet_name: "calc.tables.BoxTariffs.sheetName",
        script_key: "tariffs_box",
        marketplace: "wb",
    },
    TableDef {
        key: "Prices",
        label: "calc.tables.Prices.label",
        spreadsheet_title: "calc.tables.Prices.spreadsheetTitle",
        sheet_name: "calc.tables.Prices.sheetName",
        script_key: "prices_export",
        marketplace: "wb",
    },
    TableDef {
        key: "Advertising",
        label: "calc.tables.Advertising.label",
        spreadsheet_title: "calc.tables.Advertising.spreadsheetTitle",
        sheet_name: "calc.tables.Advertising.sheetName",
        script_key: "adv_campaigns_list",
        marketplace: "wb",
    },
    TableDef {
        key: "Funnel",
        label: "calc.tables.Funnel.label",
        spreadsheet_title: "calc.tables.Funnel.spreadsheetTitle",
        sheet_name: "calc.tables.Funnel.sheetName",
        script_key: "sales_funnel_products",
        marketplace: "wb",
    },
    TableDef {
        key: "Purchases",
        label: "calc.tables.Purchases.label",
        spreadsheet_title: "calc.tables.Purchases.spreadsheetTitle",
        sheet_name: "calc.tables.Purchases.sheetName",
        script_key: "",
        marketplace: "wb",
    },
    TableDef {
        key: "Stocks",
        label: "calc.tables.Stocks.label",
        spreadsheet_title: "calc.tables.Stocks.spreadsheetTitle",
        sheet_name: "calc.tables.Stocks.sheetName",
        script_key: "wb_stocks",
        marketplace: "wb",
    },
    TableDef {
        key: "WarehousesFbs",
        label: "calc.tables.WarehousesFbs.label",
        spreadsheet_title: "calc.tables.WarehousesFbs.spreadsheetTitle",
        sheet_name: "calc.tables.WarehousesFbs.sheetName",
        script_key: "warehouses_list",
        marketplace: "wb",
    },
    TableDef {
        key: "Feedbacks",
        label: "calc.tables.Feedbacks.label",
        spreadsheet_title: "calc.tables.Feedbacks.spreadsheetTitle",
        sheet_name: "calc.tables.Feedbacks.sheetName",
        script_key: "item_rating",
        marketplace: "wb",
    },
    TableDef {
        key: "WbSummary",
        label: "calc.tables.WbSummary.label",
        spreadsheet_title: "calc.tables.WbSummary.spreadsheetTitle",
        sheet_name: "calc.tables.WbSummary.sheetName",
        script_key: "",
        marketplace: "wb",
    },
];

pub const OZON_TABLES: &[TableDef] = &[
    TableDef {
        key: "OzProducts",
        label: "calc.tables.OzProducts.label",
        spreadsheet_title: "calc.tables.OzProducts.spreadsheetTitle",
        sheet_name: "calc.tables.OzProducts.sheetName",
        script_key: "ozon_product_list",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzProductReport",
        label: "calc.tables.OzProductReport.label",
        spreadsheet_title: "calc.tables.OzProductReport.spreadsheetTitle",
        sheet_name: "calc.tables.OzProductReport.sheetName",
        script_key: "ozon_product_report",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzPrices",
        label: "calc.tables.OzPrices.label",
        spreadsheet_title: "calc.tables.OzPrices.spreadsheetTitle",
        sheet_name: "calc.tables.OzPrices.sheetName",
        script_key: "ozon_product_prices",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzStocks",
        label: "calc.tables.OzStocks.label",
        spreadsheet_title: "calc.tables.OzStocks.spreadsheetTitle",
        sheet_name: "calc.tables.OzStocks.sheetName",
        script_key: "ozon_stocks",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzFbsStocks",
        label: "calc.tables.OzFbsStocks.label",
        spreadsheet_title: "calc.tables.OzFbsStocks.spreadsheetTitle",
        sheet_name: "calc.tables.OzFbsStocks.sheetName",
        script_key: "ozon_fbs_stocks",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzAttributes",
        label: "calc.tables.OzAttributes.label",
        spreadsheet_title: "calc.tables.OzAttributes.spreadsheetTitle",
        sheet_name: "calc.tables.OzAttributes.sheetName",
        script_key: "ozon_product_attributes",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzFboOrders",
        label: "calc.tables.OzFboOrders.label",
        spreadsheet_title: "calc.tables.OzFboOrders.spreadsheetTitle",
        sheet_name: "calc.tables.OzFboOrders.sheetName",
        script_key: "ozon_fbo_orders",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzFbsOrders",
        label: "calc.tables.OzFbsOrders.label",
        spreadsheet_title: "calc.tables.OzFbsOrders.spreadsheetTitle",
        sheet_name: "calc.tables.OzFbsOrders.sheetName",
        script_key: "ozon_fbs_orders",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzPurchases",
        label: "calc.tables.OzPurchases.label",
        spreadsheet_title: "calc.tables.OzPurchases.spreadsheetTitle",
        sheet_name: "calc.tables.OzPurchases.sheetName",
        script_key: "",
        marketplace: "ozon",
    },
    TableDef {
        key: "OzSummary",
        label: "calc.tables.OzSummary.label",
        spreadsheet_title: "calc.tables.OzSummary.spreadsheetTitle",
        sheet_name: "calc.tables.OzSummary.sheetName",
        script_key: "",
        marketplace: "ozon",
    },
];

pub use crate::calculator::catalog_metrics::metrics;
pub use crate::calculator::catalog_cols::{WB_SOURCE_COLS, oz_headers, wb_headers};

pub fn find_table(key: &str) -> Option<&'static TableDef> {
    WB_TABLES
        .iter()
        .chain(OZON_TABLES.iter())
        .find(|t| t.key.eq_ignore_ascii_case(key))
}

pub fn is_summary_table(key: &str) -> bool {
    key.eq_ignore_ascii_case(WB_SUMMARY_KEY) || key.eq_ignore_ascii_case(OZ_SUMMARY_KEY)
}

pub fn tables_for_marketplace(marketplace: &str) -> Vec<&'static TableDef> {
    let m = marketplace.trim().to_ascii_lowercase();
    match m.as_str() {
        "none" | "empty" => Vec::new(),
        "ozon" => OZON_TABLES.iter().collect(),
        "both" => WB_TABLES.iter().chain(OZON_TABLES.iter()).collect(),
        _ => WB_TABLES.iter().collect(),
    }
}

pub fn metrics_for_marketplace(marketplace: &str) -> Vec<&'static MetricDef> {
    let m = marketplace.trim().to_ascii_lowercase();
    metrics()
        .iter()
        .filter(|x| match m.as_str() {
            "none" | "empty" => false,
            "ozon" => x.marketplace == "ozon",
            "both" => true,
            _ => x.marketplace == "wb",
        })
        .collect()
}

pub fn is_table_manual(table_modes: &std::collections::HashMap<String, String>, table_key: &str) -> bool {
    if is_summary_table(table_key) {
        return false;
    }
    if let Some(mode) = table_modes
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table_key))
        .map(|(_, v)| v.as_str())
    {
        if !mode.trim().is_empty() {
            return mode.eq_ignore_ascii_case("manual");
        }
    }
    find_table(table_key)
        .map(|d| d.script_key.trim().is_empty())
        .unwrap_or(false)
}

/// Имя файла в Drive: «WB 1. Массив универсал» / «OZON 1. …».
pub fn drive_title_for(table: &TableDef) -> String {
    let is_ozon = table.marketplace.eq_ignore_ascii_case("ozon");
    let prefix = if is_ozon { "OZON" } else { "WB" };
    let list = if is_ozon { OZON_TABLES } else { WB_TABLES };
    let index = list
        .iter()
        .position(|t| t.key.eq_ignore_ascii_case(table.key))
        .map(|i| i + 1)
        .unwrap_or(1);
    let base = if table.spreadsheet_title.trim().is_empty() {
        table.label_tr()
    } else {
        table.spreadsheet_title_tr()
    };
    format!("{prefix} {index}. {base}")
}
