use crate::core::http::rate_limit::RateLimitProfile;
use std::time::Duration;

pub mod adv_campaigns_list {
    use super::*;
    pub const SCOPE: &str = "adv/v1/promotion/count|get_promotion_count";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(5, Duration::from_secs(1), Duration::from_millis(200))
            .with_429_pause(Duration::from_secs(6))
    }
}

/// `GET /api/advert/v2/adverts?ids=` — Инфо РК (до 50 id).
pub mod wb_adv_campaigns_stats_new {
    use super::*;
    pub const SCOPE: &str = "adv/v2/adverts|get_adverts_by_ids";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(5, Duration::from_secs(1), Duration::from_millis(500))
            .with_429_pause(Duration::from_secs(21))
    }
}

/// `GET /adv/v1/upd` — расходы UPD.
pub mod wb_adv_upd {
    use super::*;
    pub const SCOPE: &str = "adv_upd_expenses|upd_get";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0, 1.0, Duration::from_secs(1))
            .with_429_pause(Duration::from_secs(6))
    }
}

/// `GET /adv/v3/fullstats` — статистика РК.
pub mod wb_adv_fullstats {
    use super::*;
    pub const SCOPE: &str = "adv/v3/fullstats|get_fullstats";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(60))
    }
}

/// `POST /adv/v0/normquery/stats`.
pub mod wb_adv_norm_query_stats {
    use super::*;
    pub const SCOPE: &str = "adv/v0/normquery/stats|post_normquery_stats";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(10, Duration::from_secs(60), Duration::from_secs(6))
            .with_429_pause(Duration::from_secs(60))
    }
}

/// `POST /adv/v0/normquery/get-bids`.
pub mod wb_adv_norm_query_bids {
    use super::*;
    pub const SCOPE: &str = "adv/v0/normquery/get-bids|post_get_bids";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(5, Duration::from_secs(1), Duration::from_millis(200))
            .with_429_pause(Duration::from_secs(6))
    }
}

/// `POST /api/v2/search-report/product/search-texts`.
pub mod wb_adv_search_texts {
    use super::*;
    pub const SCOPE: &str = "search-report/product/search-texts|post";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(60))
    }
}

/// `POST /api/v2/search-report/report`.
pub mod wb_adv_search_report_main {
    use super::*;
    pub const SCOPE: &str = "search-report/report|post";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(60))
    }
}

/// `GET /api/v1/analytics/antifraud-details`.
pub mod warehouse_self_purchases {
    use super::*;
    pub const SCOPE: &str = "analytics/antifraud-details|get";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 600.0, 1.0, Duration::from_secs(600))
            .with_429_pause(Duration::from_secs(600))
    }
}

/// `GET /api/v1/analytics/goods-labeling`.
pub mod goods_labeling {
    use super::*;
    pub const SCOPE: &str = "analytics/goods-labeling|get";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::from_secs(60))
            .with_429_pause(Duration::from_secs(60))
    }
}

/// `GET /api/analytics/v1/deductions`.
pub mod analytics_deductions {
    use super::*;
    pub const SCOPE: &str = "analytics/v1/deductions|get";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::from_secs(60))
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod sales_funnel_products {
    use super::*;
    pub const SCOPE: &str = "sales_funnel_products|post_sales_funnel_products";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(21))
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod wb_stocks {
    use super::*;
    pub const SCOPE: &str = "wb_stocks|POST|/api/analytics/v1/stocks-report/seller-warehouses";
    /// `RateLimiterProfiles.Stocks.MaxInvalidResponseRetries`.
    pub const MAX_INVALID_RETRIES: u32 = 3;
    /// `PauseOnHttp5xx`.
    pub const PAUSE_ON_5XX: Duration = Duration::from_secs(20);
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(21))
            .with_429_pause(Duration::from_secs(61))
    }
}

pub mod warehouses_list {
    use super::*;
    pub const SCOPE: &str = "warehouses_list|get_warehouses";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod wb_offices {
    use super::*;
    pub const SCOPE: &str = "wb_offices|GET|/api/v3/offices";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod supplies {
    use super::*;
    pub const SCOPE: &str = "supplies|GET|/api/v3/supplies";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod supplies_fbo {
    use super::*;
    pub const SCOPE: &str = "supplies_fbo|POST|/api/v1/supplies";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod supply_info {
    use super::*;
    pub const SCOPE: &str = "supply_info|GET|/api/v3/supplies/{id}";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod supplies_goods {
    use super::*;
    pub const SCOPE: &str = "supplies_goods|GET|/api/v1/supplies/{id}/goods";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod supplies_warehouses {
    use super::*;
    pub const SCOPE: &str = "supplies_warehouses|GET|/api/v1/warehouses";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 1.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_millis(200))
    }
}

pub mod item_rating {
    use super::*;
    pub const SCOPE: &str = "item_rating|post_item_rating";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod cards_trash {
    use super::*;
    pub const SCOPE: &str = "cards_trash|get_trash_cards_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 600.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod cards_move_to_trash {
    use super::*;
    pub const SCOPE: &str = "cards_move_to_trash|move_cards_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 600.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod cards_update_dimensions {
    use super::*;
    pub const SCOPE: &str = "cards_update_dimensions|update_cards_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 6.0, 1.0, Duration::from_secs(6))
            .with_429_pause(Duration::from_secs(6))
    }
}

pub mod cards_error_list {
    use super::*;
    pub const SCOPE: &str = "cards_error_list|get_cards_error_list_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 6.0, 1.0, Duration::from_secs(6))
            .with_429_pause(Duration::from_secs(6))
    }
}

pub mod cards_move_nm {
    use super::*;
    pub const SCOPE: &str = "cards_move_nm|move_nm_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 900.0, 1.0, Duration::from_millis(900))
            .with_429_pause(Duration::from_millis(900))
    }
}

pub mod content_media_save {
    use super::*;
    pub const SCOPE: &str = "content_media_save|media_save_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 600.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod categories_parent {
    use super::*;
    pub const SCOPE: &str = "categories_parent|get_categories_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 610.0, 1.0, Duration::from_millis(610))
            .with_429_pause(Duration::from_millis(610))
    }
}

pub mod subjects_list {
    use super::*;
    pub const SCOPE: &str = "subjects_list|get_subjects_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1000.0 / 610.0, 1.0, Duration::from_millis(610))
            .with_429_pause(Duration::from_millis(610))
    }
}

pub mod wb_tnved_codes {
    use super::*;
    pub const SCOPE: &str = "wb_tnved_codes|get_tnved_by_subject";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0 / 3.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(610))
    }
}

/// `GET /api/v1/claims` — ~1 запрос / 3 с.
pub mod claims_list {
    use super::*;
    pub const SCOPE: &str = "claims_list|get_claims_paged";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(1, Duration::from_secs(3), Duration::from_secs(3))
            .with_429_pause(Duration::from_secs(3))
    }
}

/// `GET /api/v1/feedbacks/archive`.
pub mod feedbacks_archive {
    use super::*;
    pub const SCOPE: &str = "feedbacks_archive|get_feedbacks_archive";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(2.0, 2.0, Duration::from_millis(550))
            .with_429_pause(Duration::from_secs(20))
    }
}

/// `GET /api/v1/feedbacks/count`.
pub mod feedbacks_count {
    use super::*;
    pub const SCOPE: &str = "feedbacks_count|get_feedbacks_count";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(1), Duration::from_millis(400))
            .with_429_pause(Duration::from_secs(20))
    }
}

/// `GET /api/v1/calendar/promotions`.
pub mod promotions {
    use super::*;
    pub const SCOPE: &str = "promotions|get_promotions";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(10.0 / 6.0, 5.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_secs(5))
    }
}

/// `POST /api/v1/acceptance/options` — 6 запросов/мин, интервал 10 с.
pub mod acceptance_options {
    use super::*;
    pub const SCOPE: &str = "acceptance_options|get_acceptance_options";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(6, Duration::from_secs(60), Duration::from_secs(10))
            .with_429_pause(Duration::from_secs(10))
    }
}

/// `GET /api/v1/transit-tariffs`.
pub mod transit_tariffs {
    use super::*;
    pub const SCOPE: &str = "transit_tariffs|get_transit_tariffs";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(6, Duration::from_secs(60), Duration::from_secs(10))
            .with_429_pause(Duration::from_secs(10))
    }
}

/// `GET /content/v2/object/charcs/{subjectId}`.
pub mod wb_object_charcs {
    use super::*;
    pub const SCOPE: &str = "wb_object_charcs|get_subject_charcs";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(100, Duration::from_secs(60), Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}
