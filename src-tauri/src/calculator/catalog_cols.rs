//! Source columns and summary header keys (i18n).

use crate::core::i18n::t;

#[derive(Debug, Clone, Copy)]
pub struct WbSourceColDef {
    pub key: &'static str,
    pub table_key: &'static str,
    pub sheet_name_key: &'static str,
    pub column_key: &'static str,
    pub article_key: &'static str,
}

impl WbSourceColDef {
    pub fn sheet_name_tr(&self) -> String {
        t(self.sheet_name_key)
    }

    pub fn column_tr(&self) -> String {
        t(self.column_key)
    }

    pub fn article_tr(&self) -> String {
        t(self.article_key)
    }
}

pub const WB_SOURCE_COLS: &[WbSourceColDef] = &[
    WbSourceColDef {
        key: "photo",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "photo1",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "cabinet",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "calc.headers.wb.cabinet",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "name",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "calc.headers.wb.name",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "brand",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "calc.headers.wb.brand",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "vendorCode",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "calc.sourceCols.wb.vendorCodeSeller",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "category",
        table_key: "Array",
        sheet_name_key: "calc.tables.Array.sheetName",
        column_key: "calc.sourceCols.wb.subject",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "stockFbo",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.stockFboSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "stockFbs",
        table_key: "Stocks",
        sheet_name_key: "calc.tables.Stocks.sheetName",
        column_key: "calc.sourceCols.wb.stockFbsSource",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "clicks",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.clicksSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "clicksPrev",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.clicksPrevSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "cart",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.cartSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "cartPrev",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.cartPrevSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "wishlist",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.wishlistSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "orders",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.headers.wb.orders",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "ordersPrev",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.ordersPrevSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "cancels",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.headers.wb.cancels",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "cancelsPrev",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.cancelsPrevSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "sales",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.headers.wb.sales",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "salesPrev",
        table_key: "Funnel",
        sheet_name_key: "calc.tables.Funnel.sheetName",
        column_key: "calc.sourceCols.wb.salesPrevSource",
        article_key: "calc.articles.wbCyrillic",
    },
    WbSourceColDef {
        key: "rating",
        table_key: "Feedbacks",
        sheet_name_key: "calc.tables.Feedbacks.sheetName",
        column_key: "calc.sourceCols.wb.ratingSource",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "feedbacks",
        table_key: "Feedbacks",
        sheet_name_key: "calc.tables.Feedbacks.sheetName",
        column_key: "calc.sourceCols.wb.feedbacksSource",
        article_key: "calc.articles.wbLatin",
    },
    WbSourceColDef {
        key: "price",
        table_key: "Prices",
        sheet_name_key: "calc.tables.Prices.sheetName",
        column_key: "calc.headers.wb.price",
        article_key: "calc.articles.wbNmId",
    },
    WbSourceColDef {
        key: "discount",
        table_key: "Prices",
        sheet_name_key: "calc.tables.Prices.sheetName",
        column_key: "calc.sourceCols.wb.discountSource",
        article_key: "calc.articles.wbNmId",
    },
    WbSourceColDef {
        key: "priceAfterDiscount",
        table_key: "Prices",
        sheet_name_key: "calc.tables.Prices.sheetName",
        column_key: "calc.sourceCols.wb.priceAfterDiscountSource",
        article_key: "calc.articles.wbNmId",
    },
    WbSourceColDef {
        key: "purchase",
        table_key: "Purchases",
        sheet_name_key: "calc.tables.Purchases.sheetName",
        column_key: "calc.headers.wb.purchase",
        article_key: "calc.articles.wbCyrillic",
    },
];

pub const WB_HEADER_KEYS: &[&str] = &[
    "calc.headers.wb.article",
    "calc.headers.wb.cabinet",
    "calc.headers.wb.photo",
    "calc.headers.wb.name",
    "calc.headers.wb.brand",
    "calc.headers.wb.vendorPst",
    "calc.headers.wb.category",
    "calc.headers.wb.stockFbo",
    "calc.headers.wb.stockFbs",
    "calc.headers.wb.clicks",
    "calc.headers.wb.clicksPrev",
    "calc.headers.wb.cart",
    "calc.headers.wb.cartPrev",
    "calc.headers.wb.wishlist",
    "calc.headers.wb.orders",
    "calc.headers.wb.ordersPrev",
    "calc.headers.wb.cancels",
    "calc.headers.wb.cancelsPrev",
    "calc.headers.wb.sales",
    "calc.headers.wb.salesPrev",
    "calc.headers.wb.rating",
    "calc.headers.wb.feedbacks",
    "calc.headers.wb.price",
    "calc.headers.wb.discount",
    "calc.headers.wb.priceAfterDiscount",
    "calc.headers.wb.commissionFbo",
    "calc.headers.wb.commissionFbs",
    "calc.headers.wb.purchase",
    "calc.headers.wb.logistics",
    "calc.headers.wb.hasAdvertising",
    "calc.headers.wb.adSpend2w",
    "calc.headers.wb.orderCost",
    "calc.headers.wb.length",
    "calc.headers.wb.width",
    "calc.headers.wb.height",
    "calc.headers.wb.weight",
    "calc.headers.wb.volume",
    "calc.headers.wb.tnved",
    "calc.headers.wb.certNumber",
    "calc.headers.wb.certRegDate",
    "calc.headers.wb.certEndDate",
    "calc.headers.wb.certStatus",
    "calc.headers.wb.sgt",
    "calc.headers.wb.logisticsWarehouse",
    "calc.headers.wb.logisticsBase",
    "calc.headers.wb.logisticsLiter",
];

pub const OZ_HEADER_KEYS: &[&str] = &[
    "calc.headers.oz.sku",
    "calc.headers.oz.cabinet",
    "calc.headers.oz.offerId",
    "calc.headers.oz.photo",
    "calc.headers.oz.name",
    "calc.headers.oz.brand",
    "calc.headers.oz.category",
    "calc.headers.oz.stockFbo",
    "calc.headers.oz.stockFbs",
    "calc.headers.oz.rating",
    "calc.headers.oz.feedbacks",
    "calc.headers.oz.orders",
    "calc.headers.oz.ordersPrev",
    "calc.headers.oz.priceAfterDiscount",
    "calc.headers.oz.discount",
    "calc.headers.oz.priceAfterPromo",
    "calc.headers.oz.commissionFbo",
    "calc.headers.oz.commissionFbs",
    "calc.headers.oz.purchase",
    "calc.headers.oz.logisticsMin",
    "calc.headers.oz.logisticsMax",
    "calc.headers.oz.lastMile",
    "calc.headers.oz.kgt",
    "calc.headers.oz.logistics",
    "calc.headers.oz.length",
    "calc.headers.oz.width",
    "calc.headers.oz.height",
    "calc.headers.oz.weight",
    "calc.headers.oz.volume",
    "calc.headers.oz.tnved",
];

pub fn wb_headers() -> Vec<String> {
    WB_HEADER_KEYS.iter().map(|k| t(k)).collect()
}

pub fn oz_headers() -> Vec<String> {
    OZ_HEADER_KEYS.iter().map(|k| t(k)).collect()
}

