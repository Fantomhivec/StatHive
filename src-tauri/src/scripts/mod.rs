pub mod ozon_average_delivery_time;
pub mod ozon_coinvest;
pub mod ozon_commissions;
pub mod ozon_fbo_orders;
pub mod ozon_fbs_orders;
pub mod ozon_fbs_stocks;
pub mod ozon_product_attributes;
pub mod ozon_product_info;
pub mod ozon_product_list;
pub mod ozon_product_prices;
pub mod ozon_product_report;
pub mod ozon_stocks;
pub mod ozon_subscription;
pub mod wb_adv_campaigns;
pub mod wb_adv_campaigns_stats;
pub mod wb_adv_fullstats;
pub mod wb_adv_norm_query_bids;
pub mod wb_adv_norm_query_stats;
pub mod wb_adv_search_report_main;
pub mod wb_adv_search_texts;
pub mod wb_adv_upd;
pub mod wb_analytics_deductions;
pub mod wb_goods_labeling;
pub mod wb_warehouse_self_purchases;
pub mod wb_box_tariffs;
pub mod wb_cards;
pub mod wb_cards_error_list;
pub mod wb_cards_move_nm;
pub mod wb_cards_move_trash;
pub mod wb_cards_trash;
pub mod wb_cards_update;
pub mod wb_categories;
pub mod wb_commissions;
pub mod wb_content_media;
pub mod wb_item_rating;
pub mod wb_order_feed;
pub mod wb_orders_fbs;
pub mod wb_finance_util;
pub mod wb_nm_report;
pub mod wb_paid_storage;
pub mod wb_prices;
pub mod wb_prices_by_nm;
pub mod wb_prices_mutate;
pub mod wb_report_realization;
pub mod wb_sales_funnel;
pub mod wb_sales_reports_list;
pub mod wb_stock_fbo;
pub mod wb_stocks;
pub mod wb_subjects;
pub mod wb_tnved;
pub mod copy_data;
pub mod symbols_check;
pub mod cells_count;
pub mod wb_offices;
pub mod wb_supplies;
pub mod wb_acceptance_options;
pub mod wb_claims;
pub mod wb_feedbacks;
pub mod wb_object_charcs;
pub mod wb_promotions;
pub mod wb_transit_tariffs;
pub mod wb_warehouse_remains;
pub mod wb_warehouses;

pub mod common;
pub mod dates;
pub mod groups;
pub mod sheet_ranges;
pub mod ozon_list_run;
pub mod ozon_postings_run;
pub mod ozon_util;

use crate::core::script::{MemberOk, ScriptRegistry};
use crate::scripts::ozon_average_delivery_time::OzonAverageDeliveryTimeScript;
use crate::scripts::ozon_coinvest::OzonCoinvestScript;
use crate::scripts::ozon_commissions::OzonCommissionsScript;
use crate::scripts::ozon_fbo_orders::OzonFboOrdersScript;
use crate::scripts::ozon_fbs_orders::OzonFbsOrdersScript;
use crate::scripts::ozon_fbs_stocks::OzonFbsStocksScript;
use crate::scripts::ozon_product_attributes::OzonProductAttributesScript;
use crate::scripts::ozon_product_info::OzonProductInfoScript;
use crate::scripts::ozon_product_list::OzonProductListScript;
use crate::scripts::ozon_product_prices::OzonProductPricesScript;
use crate::scripts::ozon_product_report::OzonProductReportScript;
use crate::scripts::ozon_stocks::OzonStocksScript;
use crate::scripts::ozon_subscription::OzonSubscriptionScript;
use crate::scripts::wb_adv_campaigns::AdvCampaignsListScript;
use crate::scripts::wb_adv_campaigns_stats::AdvCampaignsStatsNewScript;
use crate::scripts::wb_adv_fullstats::AdvFullstatsScript;
use crate::scripts::wb_adv_norm_query_bids::AdvNormQueryBidsScript;
use crate::scripts::wb_adv_norm_query_stats::AdvNormQueryStatsScript;
use crate::scripts::wb_adv_search_report_main::AdvSearchReportMainScript;
use crate::scripts::wb_adv_search_texts::AdvSearchTextsScript;
use crate::scripts::wb_adv_upd::AdvUpdExpensesScript;
use crate::scripts::wb_analytics_deductions::AnalyticsDeductionsScript;
use crate::scripts::wb_goods_labeling::GoodsLabelingScript;
use crate::scripts::wb_warehouse_self_purchases::WarehouseSelfPurchasesScript;
use crate::scripts::wb_box_tariffs::BoxTariffsScript;
use crate::scripts::wb_cards::CardsScript;
use crate::scripts::wb_cards_error_list::CardsErrorListScript;
use crate::scripts::wb_cards_move_nm::CardsMoveNmScript;
use crate::scripts::wb_cards_move_trash::CardsMoveToTrashScript;
use crate::scripts::wb_cards_trash::CardsTrashScript;
use crate::scripts::wb_cards_update::CardsUpdateDimensionsScript;
use crate::scripts::wb_categories::CategoriesScript;
use crate::scripts::wb_commissions::CommissionsScript;
use crate::scripts::wb_content_media::ContentMediaSaveScript;
use crate::scripts::wb_item_rating::ItemRatingScript;
use crate::scripts::wb_order_feed::OrderFeedScript;
use crate::scripts::wb_orders_fbs::{OrdersFbsScript, OrdersFbsStatusScript};
use crate::scripts::wb_nm_report::{
    NmReportDownloadFileScript, NmReportDownloadsAutoScript, NmReportDownloadsListScript,
    NmReportDownloadsScript, NmReportStockHistoryAutoScript,
};
use crate::scripts::wb_paid_storage::{
    PaidStorageAutoScript, PaidStorageCreateTaskScript, PaidStorageDownloadScript,
    PaidStorageStatusScript,
};
use crate::scripts::wb_prices::PricesScript;
use crate::scripts::wb_prices_by_nm::PricesByNmScript;
use crate::scripts::wb_prices_mutate::{
    PricesHistoryTaskScript, PricesSetScript, PricesUploadScript,
};
use crate::scripts::wb_report_realization::ReportRealizationV5Script;
use crate::scripts::wb_sales_funnel::{SalesFunnelProductsHistoryScript, SalesFunnelProductsScript};
use crate::scripts::wb_sales_reports_list::SalesReportsListScript;
use crate::scripts::wb_stock_fbo::WbStockFboReportScript;
use crate::scripts::wb_stocks::WbStocksScript;
use crate::scripts::wb_subjects::SubjectsScript;
use crate::scripts::wb_tnved::WbTnvedScript;
use crate::scripts::copy_data::CopyDataScript;
use crate::scripts::symbols_check::SymbolsCheckScript;
use crate::scripts::cells_count::CellsCountScript;
use crate::scripts::wb_offices::WbOfficesScript;
use crate::scripts::wb_acceptance_options::AcceptanceOptionsScript;
use crate::scripts::wb_claims::ClaimsScript;
use crate::scripts::wb_feedbacks::{FeedbacksArchiveScript, FeedbacksCountScript};
use crate::scripts::wb_object_charcs::WbObjectCharcsScript;
use crate::scripts::wb_promotions::PromotionsScript;
use crate::scripts::wb_transit_tariffs::TransitTariffsScript;
use crate::scripts::wb_supplies::{
    SuppliesFboScript, SuppliesGoodsScript, SuppliesScript, SuppliesWarehousesScript,
    SupplyInfoScript,
};
use crate::scripts::wb_warehouse_remains::{
    WarehouseRemainsAutoScript, WarehouseRemainsCreateTaskScript, WarehouseRemainsDownloadScript,
    WarehouseRemainsStatusScript,
};
use crate::scripts::wb_warehouses::WarehousesScript;
use std::sync::Arc;

pub fn build_registry() -> ScriptRegistry {
    let mut reg = ScriptRegistry::new();
    // MemberOk = экспорт без мутаций маркетплейса (Excel-only без HasOauth).
    // Без обёртки: write/move/upload/set/create-task на API.
    reg.register(Arc::new(MemberOk(CardsScript)));
    reg.register(Arc::new(MemberOk(CardsTrashScript)));
    reg.register(Arc::new(CardsMoveToTrashScript));
    reg.register(Arc::new(CardsUpdateDimensionsScript));
    reg.register(Arc::new(MemberOk(CardsErrorListScript)));
    reg.register(Arc::new(CardsMoveNmScript));
    reg.register(Arc::new(ContentMediaSaveScript));
    reg.register(Arc::new(MemberOk(CategoriesScript)));
    reg.register(Arc::new(MemberOk(SubjectsScript)));
    reg.register(Arc::new(MemberOk(WbTnvedScript)));
    reg.register(Arc::new(MemberOk(PricesScript)));
    reg.register(Arc::new(MemberOk(PricesByNmScript)));
    reg.register(Arc::new(PricesUploadScript));
    reg.register(Arc::new(PricesSetScript));
    // create-task на API — без MemberOk (USER_ACCESS: create-task не для Excel-only).
    reg.register(Arc::new(PricesHistoryTaskScript));
    reg.register(Arc::new(MemberOk(ReportRealizationV5Script)));
    reg.register(Arc::new(MemberOk(SalesReportsListScript)));
    reg.register(Arc::new(PaidStorageCreateTaskScript));
    reg.register(Arc::new(MemberOk(PaidStorageStatusScript)));
    reg.register(Arc::new(MemberOk(PaidStorageDownloadScript)));
    reg.register(Arc::new(PaidStorageAutoScript));
    reg.register(Arc::new(MemberOk(CommissionsScript)));
    reg.register(Arc::new(MemberOk(BoxTariffsScript)));
    reg.register(Arc::new(MemberOk(AdvCampaignsListScript)));
    reg.register(Arc::new(MemberOk(AdvCampaignsStatsNewScript)));
    reg.register(Arc::new(MemberOk(AdvFullstatsScript)));
    reg.register(Arc::new(MemberOk(AdvNormQueryStatsScript)));
    reg.register(Arc::new(MemberOk(AdvNormQueryBidsScript)));
    reg.register(Arc::new(MemberOk(AdvSearchTextsScript)));
    reg.register(Arc::new(MemberOk(AdvSearchReportMainScript)));
    reg.register(Arc::new(MemberOk(AdvUpdExpensesScript)));
    reg.register(Arc::new(MemberOk(WarehouseSelfPurchasesScript)));
    reg.register(Arc::new(MemberOk(GoodsLabelingScript)));
    reg.register(Arc::new(MemberOk(AnalyticsDeductionsScript)));
    reg.register(Arc::new(MemberOk(SalesFunnelProductsScript)));
    reg.register(Arc::new(MemberOk(SalesFunnelProductsHistoryScript)));
    reg.register(Arc::new(MemberOk(WbStocksScript)));
    reg.register(Arc::new(MemberOk(WbStockFboReportScript)));
    reg.register(Arc::new(WarehouseRemainsCreateTaskScript));
    reg.register(Arc::new(MemberOk(WarehouseRemainsStatusScript)));
    reg.register(Arc::new(MemberOk(WarehouseRemainsDownloadScript)));
    reg.register(Arc::new(WarehouseRemainsAutoScript));
    reg.register(Arc::new(MemberOk(WarehousesScript)));
    reg.register(Arc::new(MemberOk(WbOfficesScript)));
    reg.register(Arc::new(MemberOk(SuppliesScript)));
    reg.register(Arc::new(MemberOk(SuppliesFboScript)));
    reg.register(Arc::new(MemberOk(SupplyInfoScript)));
    reg.register(Arc::new(MemberOk(SuppliesGoodsScript)));
    reg.register(Arc::new(MemberOk(SuppliesWarehousesScript)));
    reg.register(Arc::new(MemberOk(CopyDataScript)));
    reg.register(Arc::new(MemberOk(SymbolsCheckScript)));
    reg.register(Arc::new(MemberOk(CellsCountScript)));
    reg.register(Arc::new(MemberOk(ItemRatingScript)));
    reg.register(Arc::new(MemberOk(ClaimsScript)));
    reg.register(Arc::new(MemberOk(FeedbacksArchiveScript)));
    reg.register(Arc::new(MemberOk(FeedbacksCountScript)));
    reg.register(Arc::new(MemberOk(PromotionsScript)));
    reg.register(Arc::new(MemberOk(AcceptanceOptionsScript)));
    reg.register(Arc::new(MemberOk(TransitTariffsScript)));
    reg.register(Arc::new(MemberOk(WbObjectCharcsScript)));
    reg.register(Arc::new(MemberOk(OrderFeedScript)));
    reg.register(Arc::new(MemberOk(OrdersFbsScript)));
    reg.register(Arc::new(MemberOk(OrdersFbsStatusScript)));
    // create-task / auto — без MemberOk (USER_ACCESS).
    reg.register(Arc::new(NmReportDownloadsScript));
    reg.register(Arc::new(MemberOk(NmReportDownloadsListScript)));
    reg.register(Arc::new(MemberOk(NmReportDownloadFileScript)));
    reg.register(Arc::new(NmReportDownloadsAutoScript));
    reg.register(Arc::new(NmReportStockHistoryAutoScript));
    reg.register(Arc::new(MemberOk(OzonProductListScript)));
    reg.register(Arc::new(MemberOk(OzonProductInfoScript)));
    reg.register(Arc::new(MemberOk(OzonProductReportScript)));
    reg.register(Arc::new(MemberOk(OzonProductPricesScript)));
    reg.register(Arc::new(MemberOk(OzonCommissionsScript)));
    reg.register(Arc::new(MemberOk(OzonSubscriptionScript)));
    reg.register(Arc::new(MemberOk(OzonStocksScript)));
    reg.register(Arc::new(MemberOk(OzonFbsStocksScript)));
    reg.register(Arc::new(MemberOk(OzonProductAttributesScript)));
    reg.register(Arc::new(MemberOk(OzonFboOrdersScript)));
    reg.register(Arc::new(MemberOk(OzonFbsOrdersScript)));
    reg.register(Arc::new(MemberOk(OzonCoinvestScript)));
    reg.register(Arc::new(MemberOk(OzonAverageDeliveryTimeScript)));
    reg
}
