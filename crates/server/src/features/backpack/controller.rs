use axum::{Extension, Json, extract::State};
use backpack::services::deposit::{DepositPayload, deposit_to_backpack};
use serde::Deserialize;
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    extractors::ValidatedJson,
    features::auth::types::AuthClaims,
};

// ============================= Request Types =============================

#[derive(Debug, Deserialize, Validate)]
pub struct BackpackDepositRequest {
    /// Amount in USDC raw units (6 decimals). e.g. 10_000_000 = 10 USDC.
    #[validate(range(min = 1, message = "Amount must be greater than 0"))]
    pub amount: u64,
}

// ============================= Handlers =============================

/// POST /backpack/deposit
///
/// Fetches the Backpack USDC deposit address using server-held credentials,
/// then builds and partially signs a Solana SPL token transfer transaction
/// for the client to co-sign and submit.
///
/// Returns a base64-encoded partially-signed transaction.
pub async fn deposit(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<BackpackDepositRequest>,
) -> Result<Json<String>, AppError> {
    let deposit_payload = DepositPayload {
        user_address: claims.solana_address,
        amount: payload.amount,
    };

    let serialized_tx = deposit_to_backpack(
        state.config.solana_rpc_url,
        state.config.solana_fee_payer_private_key,
        state.config.backpack_api_key,
        state.config.backpack_api_secret,
        deposit_payload,
    )
    .await
    .map_err(|e| AppError::internal(format!("{e:?}")))?;

    Ok(Json(serialized_tx))
}
