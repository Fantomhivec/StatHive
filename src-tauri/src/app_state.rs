use crate::auth::AuthService;
use crate::core::config::AppConfig;
use crate::core::http::HttpClient;
use crate::core::log::AppLog;
use crate::core::rate_limit::KeyedRateLimiter;
use crate::core::script::ScriptRegistry;
use crate::scripts::build_registry;
use crate::updates::UpdateInfo;
use std::sync::Mutex;

pub struct AppState {
    pub log: AppLog,
    pub config: Mutex<AppConfig>,
    pub auth: AuthService,
    pub registry: ScriptRegistry,
    pub http: HttpClient,
    pub rate_limiter: KeyedRateLimiter,
    pub last_update: Mutex<Option<UpdateInfo>>,
}

impl AppState {
    pub fn new() -> Self {
        let log = AppLog::new();
        let config = AppConfig::load();
        let auth = AuthService::new(log.clone());
        let registry = build_registry();
        let http = HttpClient::new();
        let rate_limiter = KeyedRateLimiter::new();
        log.info(crate::core::i18n::t("log.appStarted"));
        Self {
            log,
            config: Mutex::new(config),
            auth,
            registry,
            http,
            rate_limiter,
            last_update: Mutex::new(None),
        }
    }
}
