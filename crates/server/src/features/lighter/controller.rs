use alloy::signers::local::PrivateKeySigner;
use axum::{Extension, Json, extract::State};
use lighter::{
    LighterExchange,
    services::deposit::{DepositPayload, PermitSignature, deposit_to_lighter},
};
use perp_core::MarketInfo;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    AppState, error::AppError, extractors::ValidatedJson, features::auth::types::AuthClaims,
};

#[derive(Debug, Serialize)]
pub struct FeePayerResponse {
    pub address: String,
}

/// Returns the EVM fee-payer address that the FE must use as the `spender` in
/// the EIP-2612 USDC permit. The user signs `permit(owner=user, spender=feePayer, …)`
/// so the fee payer can pull USDC out of the user's wallet and then call
/// `Lighter.deposit(_to=user, …)` on the user's behalf.
pub async fn get_fee_payer(
    State(state): State<AppState>,
) -> Result<Json<FeePayerResponse>, AppError> {
    let signer: PrivateKeySigner = state
        .config
        .evm_fee_payer_private_key
        .parse()
        .map_err(|e| AppError::Internal(format!("invalid EVM_FEE_PAYER_PRIVATE_KEY: {e:?}")))?;

    Ok(Json(FeePayerResponse {
        address: format!("{:?}", signer.address()),
    }))
}

pub async fn get_perp_metadata() -> Result<Json<Vec<MarketInfo>>, AppError> {
    let exchange = LighterExchange::new();
    let markets = exchange.fetch_active_perp_markets().await?;

    let response = markets
        .into_iter()
        .map(|market| MarketInfo {
            symbol: market.symbol,
            max_leverage: market.max_leverage,
            tick_size: market.tick_size,
            min_order_size: market.min_order_size,
            size_decimals: market.size_decimals,
            is_active: true,
            exchange_id: Some(market.market_index),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize, Validate)]
pub struct LighterDepositRequest {
    pub amount: String,
    pub permit: PermitSignature,
    #[serde(default)]
    pub asset_index: Option<u64>,
    #[serde(default)]
    pub route_type: Option<u64>,
}

pub async fn deposit(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LighterDepositRequest>,
) -> Result<Json<String>, AppError> {
    let ethereum_rpc_url = &state.config.ethereum_rpc_url;
    let fee_payer_private_key = state.config.evm_fee_payer_private_key;

    let deposit_payload = DepositPayload {
        user: claims.evm_address,
        amount: payload.amount,
        permit: payload.permit,
        asset_index: payload.asset_index,
        route_type: payload.route_type,
    };

    let tx_hash =
        deposit_to_lighter(ethereum_rpc_url, fee_payer_private_key, deposit_payload).await?;

    Ok(Json(tx_hash))
}
