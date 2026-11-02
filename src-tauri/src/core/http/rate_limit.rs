//! Keyed rate limiter (паритет RateLimiter): script|cabinet|scope|profile.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
    SlidingWindow,
    TokenBucket,
}

/// Профиль лимита + пауза при HTTP 429.
#[derive(Debug, Clone)]
pub struct RateLimitProfile {
    pub strategy: RateLimitStrategy,
    pub max_requests: u32,
    pub window: Duration,
    pub tokens_per_second: f64,
    pub bucket_capacity: f64,
    pub min_interval: Duration,
    pub pause_on_429: Duration,
}

impl RateLimitProfile {
    pub fn sliding_window(max_requests: u32, window: Duration, min_interval: Duration) -> Self {
        Self {
            strategy: RateLimitStrategy::SlidingWindow,
            max_requests,
            window,
            tokens_per_second: 0.0,
            bucket_capacity: 0.0,
            min_interval,
            pause_on_429: Duration::from_secs(1),
        }
    }

    pub fn token_bucket(tokens_per_second: f64, bucket_capacity: f64, min_interval: Duration) -> Self {
        Self {
            strategy: RateLimitStrategy::TokenBucket,
            max_requests: 0,
            window: Duration::ZERO,
            tokens_per_second,
            bucket_capacity,
            min_interval,
            pause_on_429: Duration::from_secs(1),
        }
    }

    pub fn with_429_pause(mut self, pause: Duration) -> Self {
        self.pause_on_429 = pause;
        self
    }

    pub fn profile_key(&self) -> String {
        match self.strategy {
            RateLimitStrategy::SlidingWindow => format!(
                "SW|{}|{}|{}",
                self.max_requests,
                self.window.as_millis(),
                self.min_interval.as_millis()
            ),
            RateLimitStrategy::TokenBucket => format!(
                "TB|{:.6}|{:.6}|{}",
                self.tokens_per_second,
                self.bucket_capacity,
                self.min_interval.as_millis()
            ),
        }
    }
}

enum LimiterKind {
    Sliding(SlidingWindow),
    Token(TokenBucket),
}

struct SlidingWindow {
    history: VecDeque<Instant>,
    max_requests: u32,
    window: Duration,
    min_interval: Duration,
    last: Option<Instant>,
}

struct TokenBucket {
    tokens_per_second: f64,
    capacity: f64,
    min_interval: Duration,
    available: f64,
    last_refill: Instant,
    last_request: Option<Instant>,
}

/// Общий пул лимитеров по ключу script|cabinet|scope|profile.
#[derive(Clone, Default)]
pub struct KeyedRateLimiter {
    inner: Arc<Mutex<HashMap<String, LimiterKind>>>,
}

impl KeyedRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ждёт слот; возвращает фактическое время ожидания (для логов скриптов).
    pub async fn wait(
        &self,
        script_key: &str,
        cabinet: &str,
        scope_key: &str,
        profile: &RateLimitProfile,
    ) -> Duration {
        let key = format!(
            "{script_key}|{cabinet}|{scope_key}|{}",
            profile.profile_key()
        );
        let started = Instant::now();
        loop {
            let delay = {
                let mut map = self.inner.lock().await;
                if !map.contains_key(&key) {
                    let lim = match profile.strategy {
                        RateLimitStrategy::SlidingWindow => LimiterKind::Sliding(SlidingWindow {
                            history: VecDeque::new(),
                            max_requests: profile.max_requests.max(1),
                            window: profile.window,
                            min_interval: profile.min_interval,
                            last: None,
                        }),
                        RateLimitStrategy::TokenBucket => LimiterKind::Token(TokenBucket {
                            tokens_per_second: profile.tokens_per_second.max(0.1),
                            capacity: profile.bucket_capacity.max(1.0),
                            min_interval: profile.min_interval,
                            available: profile.bucket_capacity.max(1.0),
                            last_refill: Instant::now(),
                            last_request: None,
                        }),
                    };
                    map.insert(key.clone(), lim);
                }
                match map.get_mut(&key).expect("rate limiter just inserted") {
                    LimiterKind::Sliding(s) => s.try_acquire(),
                    LimiterKind::Token(t) => t.try_acquire(),
                }
            };
            match delay {
                None => return started.elapsed(),
                Some(d) => sleep(d.max(Duration::from_millis(10))).await,
            }
        }
    }
}

impl SlidingWindow {
    fn try_acquire(&mut self) -> Option<Duration> {
        let now = Instant::now();
        while let Some(front) = self.history.front() {
            if now.duration_since(*front) > self.window {
                self.history.pop_front();
            } else {
                break;
            }
        }
        let by_count = (self.history.len() as u32) < self.max_requests;
        let by_interval = self
            .last
            .map(|t| now.duration_since(t) >= self.min_interval)
            .unwrap_or(true);
        if by_count && by_interval {
            self.history.push_back(now);
            self.last = Some(now);
            return None;
        }
        let mut delay = Duration::from_millis(20);
        if !by_interval {
            if let Some(last) = self.last {
                let until = self
                    .min_interval
                    .saturating_sub(now.duration_since(last));
                if until > delay {
                    delay = until;
                }
            }
        }
        if !by_count {
            if let Some(oldest) = self.history.front() {
                let until = self
                    .window
                    .saturating_sub(now.duration_since(*oldest))
                    + Duration::from_millis(10);
                if until > delay {
                    delay = until;
                }
            }
        }
        Some(delay.min(Duration::from_secs(5)))
    }
}

impl TokenBucket {
    fn try_acquire(&mut self) -> Option<Duration> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.available = (self.available + elapsed * self.tokens_per_second).min(self.capacity);
            self.last_refill = now;
        }
        let by_interval = self
            .last_request
            .map(|t| now.duration_since(t) >= self.min_interval)
            .unwrap_or(true);
        if self.available >= 1.0 && by_interval {
            self.available -= 1.0;
            self.last_request = Some(now);
            return None;
        }
        Some(Duration::from_millis(50))
    }
}
