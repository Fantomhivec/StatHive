//! Внутренние типы метода HTTP для `RequestSender`.

use super::rate_limit::RateLimitProfile;
use serde_json::Value;

#[derive(Clone, Copy)]
pub enum Auth<'a> {
    Bearer(&'a str),
    Raw(&'a str),
}

#[derive(Clone, Copy)]
pub struct Limit<'a> {
    pub cabinet: &'a str,
    pub scope: &'a str,
    pub profile: &'a RateLimitProfile,
}

/// Путь URL без query — для коротких логов запросов.
pub(crate) fn url_path_for_log(url: &str) -> String {
    let no_q = url.split('?').next().unwrap_or(url);
    if let Some(rest) = no_q.split("://").nth(1) {
        if let Some(i) = rest.find('/') {
            return rest[i..].to_string();
        }
    }
    no_q.to_string()
}

pub(crate) enum Method<'a> {
    Get {
        url: &'a str,
        auth: Auth<'a>,
    },
    Post {
        url: &'a str,
        auth: Auth<'a>,
        body: Option<&'a Value>,
    },
}

impl Method<'_> {
    pub(crate) fn clone_owned(&self) -> OwnedMethod {
        match self {
            Method::Get { url, auth } => {
                let (bearer, raw) = auth_parts(*auth);
                OwnedMethod::Get {
                    url: (*url).to_string(),
                    bearer,
                    raw,
                }
            }
            Method::Post { url, auth, body } => {
                let (bearer, raw) = auth_parts(*auth);
                OwnedMethod::Post {
                    url: (*url).to_string(),
                    bearer,
                    raw,
                    body: body.cloned().unwrap_or(Value::Null),
                }
            }
        }
    }
}

pub(crate) enum OwnedMethod {
    Get {
        url: String,
        bearer: Option<String>,
        raw: bool,
    },
    Post {
        url: String,
        bearer: Option<String>,
        raw: bool,
        body: Value,
    },
}

pub(crate) fn auth_parts(auth: Auth<'_>) -> (Option<String>, bool) {
    match auth {
        Auth::Bearer(t) => (Some(t.to_string()), false),
        Auth::Raw(t) => (Some(t.to_string()), true),
    }
}
