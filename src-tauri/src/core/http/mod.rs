//! HTTP-клиент + keyed rate limit + RequestSender для marketplace API.

pub mod client;
pub mod rate_limit;
pub mod rate_profiles;
pub mod request_sender;
mod request_sender_method;

pub use client::HttpClient;
pub use rate_limit::KeyedRateLimiter;
pub use request_sender::RequestSender;
