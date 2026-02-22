use crate::features::auth::types::AuthClaims;
use crate::middleware::bridge::{validate_balance, validate_destination_usdc_address};
use crate::middleware::user::validate_evm_address;
use crate::state::AppState;
use axum::extract::State;
use axum::{Extension, Json};
use bridge::client::{BridgeClient, PermitRequest, QuoteRequest, QuoteResponse};
use perp_core::Chain;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_destination_usdc_address"))]
pub struct QuotePayload {
    #[validate(custom(function = "validate_evm_address"))]
    pub user: String,
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
    #[validate(length(min = 1, message = "Amount must not be empty"))]
    pub amount: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "usePermit")]
    pub use_permit: bool,
    #[validate(length(min = 1, message = "Recipient must not be empty"))]
    pub recipient: String,
}

//* Bridge only supports base to other chains for now */
pub async fn get_quote(
    State(app_state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<QuotePayload>,
) -> Result<Json<QuoteResponse>, AppError> {
    payload.validate()?;

    if !payload.user.eq_ignore_ascii_case(&claims.evm_address) {
        return Err(AppError::unauthorised(
            "payload.user does not match authenticated EVM address",
        ));
    }

    validate_balance(&payload, &app_state.config.base_rpc_url).await.map_err(|e| {
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

    let relay_api_key = app_state.config.relay_api_key;
    let bridge_client = BridgeClient::new(relay_api_key);

    let quote = bridge_client.quote(quote_request).await?;

    Ok(Json(quote))
}

pub async fn execute_permits(
    State(app_state): State<AppState>,
    Json(payload): Json<PermitRequest>,
) -> Result<Json<String>, AppError> {
    let mut errors = validator::ValidationErrors::new();

    if payload.signature.is_empty() {
        let mut err = validator::ValidationError::new("required");
        err.message = Some("Signature must not be empty".into());
        errors.add("signature", err);
    }
    if payload.kind.is_empty() {
        let mut err = validator::ValidationError::new("required");
        err.message = Some("Kind must not be empty".into());
        errors.add("kind", err);
    }
    if payload.request_id.is_empty() {
        let mut err = validator::ValidationError::new("required");
        err.message = Some("Request ID must not be empty".into());
        errors.add("request_id", err);
    }

    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let relay_api_key = app_state.config.relay_api_key;
    let bridge_client = BridgeClient::new(relay_api_key);

    let result = bridge_client.execute_permit(payload).await?;

    let serialized_response = serde_json::to_string(&result)?;

    Ok(Json(serialized_response))
}
