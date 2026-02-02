use axum::Json;
use bridge::client::{BridgeClient, PermitRequest, QuoteRequest};

use crate::error::AppError;

pub async fn get_quote(Json(payload): Json<QuoteRequest>) -> Result<Json<String>, AppError> {
    let bridge_client = BridgeClient::new();

    let quote = bridge_client.quote(payload).await?;

    let serialized_response = serde_json::to_string(&quote)?;

    Ok(Json(serialized_response))
}

pub async fn execute_permits(Json(payload): Json<PermitRequest>) -> Result<Json<String>, AppError> {
    let bridge_client = BridgeClient::new();

    let result = bridge_client.execute_permit(payload).await?;

    let serialized_response = serde_json::to_string(&result)?;

    Ok(Json(serialized_response))
}
