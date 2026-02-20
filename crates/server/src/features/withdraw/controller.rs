use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use pacifica::services::withdraw::WithdrawRequest;
use perp_core::{Chain, exchange::PerpetualExchange};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    features::{auth::types::AuthClaims, withdraw::services::WithdrawService},
    middleware::user::{validate_evm_address, validate_solana_address},
    services::withdraw::NextActionResponse,
    state::AppState,
};
use db::withdraw::{self as withdraw_db};

// ============================= Request / Response Types =============================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateWithdrawalIntentRequest {
    pub user_id: Uuid,
    pub exchange: PerpetualExchange,
    #[validate(range(exclusive_min = 0.0, message = "Amount must be greater than 0"))]
    pub amount_usd: f64,
    #[validate(custom(function = "validate_evm_address"))]
    pub evm_address: String,
    #[validate(custom(function = "validate_evm_address"))]
    pub recipient: String,
    /// Chain ID of the destination (defaults to Base = 8453).
    #[serde(default = "default_destination_chain_id")]
    pub destination_chain_id: i32,
}

fn default_destination_chain_id() -> i32 {
    Chain::BASE.id as i32
}

#[derive(Debug, Serialize)]
pub struct CreateWithdrawalIntentResponse {
    pub withdrawal_intent_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ActionResultRequest {
    #[validate(length(min = 1, message = "Action must not be empty"))]
    pub action: String,
    #[serde(default)]
    pub success: bool,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActionResultResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct WithdrawalIntentDetailResponse {
    pub intent: withdraw_db::WithdrawalIntent,
    pub steps: Vec<withdraw_db::WithdrawalStep>,
}

// ============================= Handlers =============================

/// POST /withdraw-intents — Create a new withdrawal intent.
pub async fn create_withdrawal_intent(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateWithdrawalIntentRequest>,
) -> Result<Json<CreateWithdrawalIntentResponse>, AppError> {
    payload.validate()?;

    if !payload
        .evm_address
        .eq_ignore_ascii_case(&claims.evm_address)
    {
        return Err(AppError::unauthorised(
            "payload.evm_address does not match authenticated EVM address",
        ));
    }

    let user_id = payload.user_id;

    WithdrawService::create_withdrawal_intent(state.db, payload, user_id)
        .await
        .map(|id| {
            Json(CreateWithdrawalIntentResponse {
                withdrawal_intent_id: id,
            })
        })
}

/// GET /withdraw-intents/:id — Get the full intent + steps detail.
pub async fn get_withdrawal_intent_detail(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<WithdrawalIntentDetailResponse>, AppError> {
    let intent = withdraw_db::get_withdrawal_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Withdrawal intent {intent_id}")))?;

    let steps = withdraw_db::get_withdrawal_steps(state.db, intent_id).await?;

    Ok(Json(WithdrawalIntentDetailResponse { intent, steps }))
}

/// GET /withdraw-intents/:id/next-action — Compute and return the next action for the client.
pub async fn get_next_action(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<NextActionResponse>, AppError> {
    WithdrawService::get_next_action(state.db, intent_id)
        .await
        .map(Json)
}

/// POST /withdraw-intents/:id/action-result — Client reports the outcome of an executed action.
pub async fn report_action_result(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
    Json(payload): Json<ActionResultRequest>,
) -> Result<Json<ActionResultResponse>, AppError> {
    payload.validate()?;

    let intent = withdraw_db::get_withdrawal_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Withdrawal intent {intent_id}")))?;

    if !intent.evm_address.eq_ignore_ascii_case(&claims.evm_address) {
        return Err(AppError::unauthorised(
            "withdrawal intent does not belong to authenticated user",
        ));
    }

    WithdrawService::report_action_result(state.db, intent_id, payload)
        .await
        .map(|action| {
            Json(ActionResultResponse {
                status: "accepted".to_string(),
                message: format!("Action result for {action} processed"),
            })
        })
}

/// GET /withdraw-intents/user/:user_id — List all withdrawal intents for a user.
pub async fn list_user_withdrawal_intents(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<withdraw_db::WithdrawalIntent>>, AppError> {
    let intents = withdraw_db::get_withdrawal_intents_by_user(state.db, user_id).await?;
    Ok(Json(intents))
}

// ============================= Withdraw Transaction =============================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HyperliquidTransactionRequest {
    /// The user's address on the source chain (must match auth claims).
    #[validate(custom(function = "validate_evm_address"))]
    pub evm_address: String,
    /// Where the withdrawn USDC should land (e.g. user's Arbitrum address for Hyperliquid).
    #[validate(custom(function = "validate_evm_address"))]
    pub destination: String,
    /// Amount in USDC as a decimal string (e.g. "100.5").
    #[validate(length(min = 1, message = "Amount must not be empty"))]
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PacificaTransactionRequest {
    #[validate(custom(function = "validate_solana_address"))]
    pub account: String,
    pub signature: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateWithdrawTransactionRequest {
    Hyperliquid(HyperliquidTransactionRequest),
    Pacifica(PacificaTransactionRequest),
}

/// POST /withdraw-intents/transaction — Generate the signed withdrawal transaction data.
///
/// For Hyperliquid this returns EIP-712 typed data the client must sign and submit
/// to Hyperliquid's /exchange endpoint. The server never holds the user's private key.
pub async fn create_withdraw_transaction(
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<CreateWithdrawTransactionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    //todo
    // payload.validate()?;

    match payload {
        CreateWithdrawTransactionRequest::Hyperliquid(payload) => {
            if !payload
                .evm_address
                .eq_ignore_ascii_case(&claims.evm_address)
            {
                return Err(AppError::unauthorised(
                    "payload.evm_address does not match authenticated EVM address",
                ));
            }
            let response =
                hyperliquid::services::withdraw(Some(payload.destination), payload.amount)
                    .await
                    .map_err(|e| AppError::internal(format!("{e:?}")))?;

            let value = serde_json::to_value(response)?;
            Ok(Json(value))
        }
        CreateWithdrawTransactionRequest::Pacifica(payload) => {
            let withdrawal_request = WithdrawRequest {
                account: payload.account,
                signature: payload.signature,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                amount: payload.amount,
            };

            let response = pacifica::services::withdraw::withdraw(withdrawal_request).await?;

            let value = serde_json::to_value(&response)?;
            Ok(Json(value))
        }
        _ => unimplemented!(),
    }
}
