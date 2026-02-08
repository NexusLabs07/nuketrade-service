//! Exchange-specific error types with detailed context.

use thiserror::Error;

/// Exchange-specific errors with detailed context.
#[derive(Debug, Error)]
pub enum ExchangeError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error in field '{field}': {message}")]
    Parse { field: String, message: String },

    #[error("API error: {message} (code: {code:?})")]
    Api {
        message: String,
        code: Option<String>,
    },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Address required but not provided")]
    AddressRequired,

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl ExchangeError {
    /// Create a parse error with field context.
    pub fn parse(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an API error.
    pub fn api(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Api {
            message: message.into(),
            code,
        }
    }
}
