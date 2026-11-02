use crate::core::http::rate_limit::RateLimitProfile;
use std::time::Duration;

fn ozon_default(_scope: &'static str) -> RateLimitProfile {
    RateLimitProfile::token_bucket(50.0, 10.0, Duration::from_millis(20))
        .with_429_pause(Duration::from_secs(1))
}

/// sliding window 50 req / 1 s / min 20 ms.
fn ozon_sw_50() -> RateLimitProfile {
    RateLimitProfile::sliding_window(50, Duration::from_secs(1), Duration::from_millis(20))
        .with_429_pause(Duration::from_secs(1))
}

pub mod product_list {
    use super::*;
    pub const SCOPE: &str = "ozon_product_list|post_product_list";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod product_report {
    use super::*;
    pub const SCOPE: &str = "ozon_product_report|report";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod product_prices {
    use super::*;
    pub const SCOPE: &str = "ozon_product_prices|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod stocks {
    use super::*;
    pub const SCOPE: &str = "ozon_stocks|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod product_info {
    use super::*;
    pub const SCOPE: &str = "ozon_product_info|v3_product_info_list";
    pub fn profile() -> RateLimitProfile {
        ozon_sw_50()
    }
}
pub mod commissions {
    use super::*;
    pub const SCOPE: &str = "ozon_commissions|v3_product_info_list";
    pub fn profile() -> RateLimitProfile {
        ozon_sw_50()
    }
}
pub mod subscription {
    use super::*;
    pub const SCOPE: &str = "ozon_subscription|v1_product_info_subscription";
    pub fn profile() -> RateLimitProfile {
        ozon_sw_50()
    }
}
pub mod average_delivery_time {
    use super::*;
    pub const SCOPE: &str = "ozon_average_delivery_time|v1_average_delivery_time_summary";
    pub fn profile() -> RateLimitProfile {
        ozon_sw_50()
    }
}
pub mod fbs_stocks {
    use super::*;
    pub const SCOPE: &str = "ozon_fbs_stocks|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod product_attributes {
    use super::*;
    pub const SCOPE: &str = "ozon_product_attributes|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod fbo_orders {
    use super::*;
    pub const SCOPE: &str = "ozon_fbo_orders|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod fbs_orders {
    use super::*;
    pub const SCOPE: &str = "ozon_fbs_orders|post";
    pub fn profile() -> RateLimitProfile {
        ozon_default(SCOPE)
    }
}
pub mod download {
    use super::*;
    pub const SCOPE: &str = "ozon|download_bytes";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0, 2.0, Duration::from_millis(200))
            .with_429_pause(Duration::from_secs(2))
    }
}
/// GET /v1/actions — 1 req / 5 s.
pub mod coinvest_actions {
    use super::*;
    pub const SCOPE: &str = "ozon_coinvest|v1_actions";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(1, Duration::from_secs(5), Duration::from_secs(5))
            .with_429_pause(Duration::from_secs(2))
    }
}
/// POST /v2/actions/candidates — 1 req / 1.5 s.
pub mod coinvest_candidates {
    use super::*;
    pub const SCOPE: &str = "ozon_coinvest|v2_actions_candidates";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(
            1,
            Duration::from_millis(1500),
            Duration::from_millis(1500),
        )
        .with_429_pause(Duration::from_secs(2))
    }
}
