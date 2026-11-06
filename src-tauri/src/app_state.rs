// Copyright 2026 StatHive
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
