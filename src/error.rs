#![allow(dead_code)] // вариантов сейчас больше чем использований; подчищается в шагах 2+.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("http error: {0}")]
    Http(#[from] Box<ureq::Error>),

    #[error("rate limited (HTTP 429, retry_after_ms={retry_after_ms:?})")]
    RateLimited { retry_after_ms: Option<u64> },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("api error (code={code}): {errors:?}")]
    Api { code: i32, errors: Vec<String> },

    #[error("other: {0}")]
    Other(#[from] anyhow::Error),
}

impl From<ureq::Error> for BotError {
    fn from(e: ureq::Error) -> Self {
        BotError::Http(Box::new(e))
    }
}

pub type Result<T> = std::result::Result<T, BotError>;
