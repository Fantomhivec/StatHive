use crate::core::http::rate_limit::RateLimitProfile;
use std::time::Duration;

pub mod cards_list {
    use super::*;
    pub const SCOPE: &str = "cards_list|cards_list_request";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(100, Duration::from_secs(60), Duration::from_secs(1))
            .with_429_pause(Duration::from_secs(61))
    }
}

pub mod prices_export {
    use super::*;
    pub const SCOPE: &str = "prices_export|get_prices_paged";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0 / 3.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod prices_by_nm {
    use super::*;
    pub const SCOPE: &str = "prices_by_nm|get_prices_for_nm_list";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(10, Duration::from_secs(6), Duration::from_millis(600))
            .with_429_pause(Duration::from_secs(2))
    }
}

pub mod report_realization_v5 {
    use super::*;
    pub const SCOPE: &str = "report_realization_v5|get_report_detail_by_period";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 61.0, 1.0, Duration::from_secs(61))
            .with_429_pause(Duration::from_secs(61))
    }
}

pub mod prices_upload {
    use super::*;
    pub const SCOPE: &str = "prices_upload|upload_task";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0 / 3.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod prices_set {
    use super::*;
    pub const SCOPE: &str = "prices_set|upload_task";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0 / 3.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod prices_history_task {
    use super::*;
    pub const SCOPE: &str = "prices_history_task|get_history_goods_task";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(5.0 / 3.0, 1.0, Duration::from_millis(600))
            .with_429_pause(Duration::from_millis(600))
    }
}

pub mod paid_storage_create_task {
    use super::*;
    pub const SCOPE: &str = "paid_storage_create_task|create_paid_storage_task";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::from_secs(60))
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod paid_storage_status {
    use super::*;
    pub const SCOPE: &str = "paid_storage_task_status|get_paid_storage_task_status";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 5.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(5))
    }
}

pub mod paid_storage_download {
    use super::*;
    pub const SCOPE: &str = "paid_storage_download|get_paid_storage_report";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::token_bucket(1.0 / 60.0, 1.0, Duration::ZERO)
            .with_429_pause(Duration::from_secs(60))
    }
}

pub mod paid_storage_auto {
    use super::*;
    pub const SCOPE_CREATE: &str = "paid_storage_auto|create_paid_storage_task";
    pub const SCOPE_STATUS: &str = "paid_storage_auto|get_paid_storage_task_status";
    pub const SCOPE_DOWNLOAD: &str = "paid_storage_auto|download_paid_storage_report";
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

pub mod tariffs_commission {
    use super::*;
    pub const SCOPE: &str = "tariffs_commission|get_tariffs_commission";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(2, Duration::from_secs(60), Duration::ZERO)
            .with_429_pause(Duration::from_secs(6))
    }
}

pub mod tariffs_box {
    use super::*;
    pub const SCOPE: &str = "tariffs_box|get_tariffs_box";
    pub fn profile() -> RateLimitProfile {
        RateLimitProfile::sliding_window(60, Duration::from_secs(60), Duration::from_secs(1))
            .with_429_pause(Duration::from_secs(6))
    }
}

