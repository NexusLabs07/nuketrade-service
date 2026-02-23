use crate::error::AppError;
use crate::extractors::ValidatedJson;
use crate::features::auth::types::AuthClaims;
use crate::features::bridge::types::{ExecutePermitPayload, QuotePayload};
use crate::state::AppState;
use crate::validation::bridge::validate_balance;

use axum::extract::State;
use axum::{Extension, Json};
use bridge::client::{BridgeClient, PermitRequest, QuoteRequest, QuoteResponse};
use perp_core::Chain;

impl From<ExecutePermitPayload> for PermitRequest {
    fn from(value: ExecutePermitPayload) -> Self {
        Self {
            signature: value.signature,
            kind: value.kind,
            request_id: value.request_id,
            api: value.api,
        }
    }
}

//* Bridge only supports base to other chains for now */
pub async fn get_quote(
    State(app_state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    ValidatedJson(payload): ValidatedJson<QuotePayload>,
) -> Result<Json<QuoteResponse>, AppError> {
    validate_balance(
        claims.evm_address.clone(),
        &payload.amount,
        &app_state.config.base_rpc_url,
    )
    .await
    .map_err(|e| {
        let mut errors = validator::ValidationErrors::new();
        errors.add("amount", e);
        AppError::Validation(errors)
    })?;

    //Deposit only available from base as of now
    let quote_request = QuoteRequest {
        user: claims.evm_address,
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
    ValidatedJson(payload): ValidatedJson<ExecutePermitPayload>,
) -> Result<Json<String>, AppError> {
    let relay_api_key = app_state.config.relay_api_key;
    let bridge_client = BridgeClient::new(relay_api_key);

    let result = bridge_client.execute_permit(payload.into()).await?;

    let serialized_response = serde_json::to_string(&result)?;

    Ok(Json(serialized_response))
}
