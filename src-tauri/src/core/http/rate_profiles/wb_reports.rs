use crate::core::http::rate_limit::RateLimitProfile;
use std::time::Duration;

/// WB order-feed: 1 req / 1 min per cabinet ( RateLimiterProfiles.OrderFeed).
pub mod order_feed {
    use super::*;
    pub const SCOPE: &str = "order_feed|POST|/api/analytics/v1/order-feed";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(1, Duration::from_secs(60), Duration::from_secs(60))
            .with_429_pause(Duration::from_secs(65))
    }
}

/// Marketplace FBS orders list: ~300 req / min.
pub mod orders_fbs {
    use super::*;
    pub const SCOPE: &str = "orders_fbs|GET|/api/v3/orders";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(300, Duration::from_secs(60), Duration::from_millis(200))
            .with_429_pause(Duration::from_secs(5))
    }
}

pub mod orders_fbs_status {
    use super::*;
    pub const SCOPE: &str = "orders_fbs_status|POST|/api/v3/orders/status";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(300, Duration::from_secs(60), Duration::from_millis(200))
            .with_429_pause(Duration::from_secs(5))
    }
}

pub mod sales_funnel_products_history {
    use super::*;
    pub const SCOPE: &str = "sales_funnel_products_history|POST|/api/analytics/v3/sales-funnel/products/history";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(25))
    }
}

/// Nm-report CSV chain: 3 req / min.
pub mod nm_report {
    use super::*;
    fn nm_sw() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(25))
    }
    pub mod create {
        use super::*;
        pub const SCOPE: &str = "nm_report_downloads|POST|/api/v2/nm-report/downloads";
        pub fn profile() -> RateLimitProfile {
            nm_sw()
        }
    }
    pub mod list {
        use super::*;
        pub const SCOPE: &str = "nm_report_downloads_list|GET|/api/v2/nm-report/downloads";
        pub fn profile() -> RateLimitProfile {
            nm_sw()
        }
    }
    pub mod file {
        use super::*;
        pub const SCOPE: &str = "nm_report_download_file|GET|/api/v2/nm-report/downloads/file";
        pub fn profile() -> RateLimitProfile {
            nm_sw()
        }
    }
    pub mod auto {
        use super::*;
        pub const SCOPE_CREATE: &str = "nm_report_downloads_auto|create";
        pub const SCOPE_LIST: &str = "nm_report_downloads_auto|list";
        pub const SCOPE_FILE: &str = "nm_report_downloads_auto|file";
        pub fn profile_create() -> RateLimitProfile {
            nm_sw()
        }
        pub fn profile_list() -> RateLimitProfile {
            nm_sw()
        }
        pub fn profile_file() -> RateLimitProfile {
            nm_sw()
        }
    }
    pub mod stock_history_auto {
        use super::*;
        pub const SCOPE_CREATE: &str = "nm_report_stock_history_auto|create";
        pub const SCOPE_LIST: &str = "nm_report_stock_history_auto|list";
        pub const SCOPE_FILE: &str = "nm_report_stock_history_auto|file";
        pub fn profile_create() -> RateLimitProfile {
            nm_sw()
        }
        pub fn profile_list() -> RateLimitProfile {
            nm_sw()
        }
        pub fn profile_file() -> RateLimitProfile {
            nm_sw()
        }
    }
}

pub mod wb_stock_fbo_report {
    use super::*;
    pub const SCOPE: &str = "wb_stock_fbo_report|POST|/api/analytics/v1/stocks-report/wb-warehouses";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(3, Duration::from_secs(60), Duration::from_secs(20))
            .with_429_pause(Duration::from_secs(25))
    }
}

pub mod warehouse_remains_create_task {
    use super::*;
    pub const SCOPE: &str = "warehouse_remains_create_task|GET|/api/v1/warehouse_remains";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::from_secs(60))
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod warehouse_remains_status {
    use super::*;
    pub const SCOPE: &str =
        "warehouse_remains_status|GET|/api/v1/warehouse_remains/tasks/{id}/status";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 5.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(5))
    }
}

pub mod warehouse_remains_download {
    use super::*;
    pub const SCOPE: &str =
        "warehouse_remains_download|GET|/api/v1/warehouse_remains/tasks/{id}/download";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod warehouse_remains_auto {
    use super::*;
    pub const SCOPE_CREATE: &str = "warehouse_remains_auto|create";
    pub const SCOPE_STATUS: &str = "warehouse_remains_auto|status";
    pub const SCOPE_DOWNLOAD: &str = "warehouse_remains_auto|download";
    pub fn profile_create() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(60))
    }
    pub fn profile_status() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 5.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(5))
    }
    pub fn profile_download() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(60))
    }
}

