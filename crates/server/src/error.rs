use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

#[derive(Debug)]
pub enum AppError {
    ValidationError(ValidationErrors),
    DatabaseError(sqlx::Error),
    InternalServerError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ValidationError(errors) => {
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

                (
                    StatusCode::BAD_REQUEST,
                    json!({
                        "error": "Validation failed",
                        "details": error_map
                    }),
                )
            }
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "error": "Database error occurred",
                        "message": "Failed to process request"
                    }),
                )
            }
            AppError::InternalServerError(msg) => {
                tracing::error!("Internal server error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "error": "Internal server error",
                        "message": msg
                    }),
                )
            }
        };

        (status, Json(error_message)).into_response()
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        AppError::ValidationError(errors)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::DatabaseError(error)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::InternalServerError(error.to_string())
    }
}
