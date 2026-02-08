use crate::middleware::bridge::{validate_balance, validate_destination_usdc_address};
use axum::Json;
use bridge::client::{BridgeClient, PermitRequest, QuoteRequest, QuoteResponse};
use perp_core::Chain;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_destination_usdc_address"))]
pub struct QuotePayload {
    pub user: String,
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
    pub amount: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "usePermit")]
    pub use_permit: bool,
    pub recipient: String,
}

pub async fn get_quote(Json(payload): Json<QuotePayload>) -> Result<Json<QuoteResponse>, AppError> {
    payload.validate()?;

    validate_balance(&payload).await.map_err(|e| {
        let mut errors = validator::ValidationErrors::new();
        errors.add("amount", e);
        AppError::Validation(errors)
    })?;

    //Deposit only available from base as of now
    let quote_request = QuoteRequest {
        user: payload.user,
        origin_chain_id: Chain::BASE.id,
        destination_chain_id: payload.destination_chain_id,
        origin_currency: Chain::BASE.usdc_address.to_string(),
        destination_currency: Chain::from_id(payload.destination_chain_id)
            .unwrap()
            .usdc_address
            .to_string(), //already verified at validation layer
        amount: payload.amount,
        trade_type: payload.trade_type,
        use_permit: payload.use_permit,
        recipient: payload.recipient,
    };

    let bridge_client = BridgeClient::new();

    let quote = bridge_client.quote(quote_request).await?;

    Ok(Json(quote))
}

pub async fn execute_permits(Json(payload): Json<PermitRequest>) -> Result<Json<String>, AppError> {
    let bridge_client = BridgeClient::new();

    let result = bridge_client.execute_permit(payload).await?;

    let serialized_response = serde_json::to_string(&result)?;

    Ok(Json(serialized_response))
}
