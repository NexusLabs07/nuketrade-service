use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use perp_core::ExchangeError;
use serde_json::json;
use validator::ValidationErrors;

/// Application error types with detailed context.
///
/// Each variant maps to appropriate HTTP status codes and provides
/// structured error responses.
#[derive(Debug)]
pub enum AppError {
    /// Validation errors from request payload validation.
    Validation(ValidationErrors),

    /// Database operation errors.
    Database(sqlx::Error),

    /// Exchange API errors (upstream service failures).
    Exchange(ExchangeError),

    /// Parse errors for specific fields.
    Parse { field: String, message: String },

    /// Network/HTTP errors when calling external services.
    Network(String),

    /// Configuration errors.
    Config(String),

    /// Resource not found.
    NotFound(String),

    /// Rate limit exceeded.
    RateLimited,

    /// Generic internal server error.
    Internal(String),
}

impl AppError {
    /// Create a parse error with field context.
    pub fn parse(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a not found error.
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// Create a network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Validation(_) => write!(f, "Validation error"),
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::Exchange(e) => write!(f, "Exchange error: {}", e),
            AppError::Parse { field, message } => write!(f, "Parse error in {}: {}", field, message),
            AppError::Network(msg) => write!(f, "Network error: {}", msg),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AppError::NotFound(resource) => write!(f, "Not found: {}", resource),
            AppError::RateLimited => write!(f, "Rate limit exceeded"),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, error_message) = match &self {
            AppError::Validation(errors) => {
                let error_map: Vec<String> = errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(move |error| {
                            format!(
                                "{}: {}",
                                field,
                                error
                                    .message
                                    .clone()
                                    .unwrap_or_else(|| "Invalid value".into())
                            )
                        })
                    })
                    .collect();

                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "validation_error",
                        "message": "Validation failed",
                        "details": error_map
                    })),
                )
                    .into_response();
            }

            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "Database operation failed".to_string(),
                )
            }

            AppError::Exchange(err) => {
                tracing::warn!("Exchange error: {:?}", err);
                (
                    StatusCode::BAD_GATEWAY,
                    "exchange_error",
                    err.to_string(),
                )
            }

            AppError::Parse { field, message } => (
                StatusCode::BAD_REQUEST,
                "parse_error",
                format!("Invalid value for {}: {}", field, message),
            ),

            AppError::Network(msg) => {
                tracing::error!("Network error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    "network_error",
                    msg.clone(),
                )
            }

            AppError::Config(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "config_error",
                    "Configuration error".to_string(),
                )
            }

            AppError::NotFound(resource) => (
                StatusCode::NOT_FOUND,
                "not_found",
                format!("Resource not found: {}", resource),
            ),

            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Rate limit exceeded. Please try again later.".to_string(),
            ),

            AppError::Internal(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
        };

        (
            status,
            Json(json!({
                "error": error_type,
                "message": error_message
            })),
        )
            .into_response()
    }
}

// From implementations for automatic error conversion

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        AppError::Validation(errors)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::Database(error)
    }
}

impl From<ExchangeError> for AppError {
    fn from(error: ExchangeError) -> Self {
        AppError::Exchange(error)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::Internal(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::Network(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Parse {
            field: "json".to_string(),
            message: error.to_string(),
        }
    }
}
