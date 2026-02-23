use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use bridge::client::{BridgeClient, QuoteRequest, QuoteResponse};
use pacifica::services::withdraw::WithdrawRequest;
use perp_core::Chain;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    features::{
        auth::types::AuthClaims,
        hedge::types::{ActionResultRequest, ActionResultResponse},
        withdraw::types::{
            BridgeRequest, CreateWithdrawTransactionRequest, CreateWithdrawalIntentRequest,
            CreateWithdrawalIntentResponse, WithdrawService, WithdrawalIntentDetailResponse,
        },
    },
    helpers::withdraw::NextActionResponse,
    state::AppState,
};
use db::withdraw::{self as withdraw_db};

// ============================= Handlers =============================

/// POST /withdraw-intents — Create a new withdrawal intent.
pub async fn create_withdrawal_intent(
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateWithdrawalIntentRequest>,
) -> Result<Json<CreateWithdrawalIntentResponse>, AppError> {
    payload.validate()?;

    let user_id = Uuid::parse_str(&claims.user_id)
        .map_err(|_| AppError::unauthorised("authenticated suborg_id is not a valid UUID"))?;

    WithdrawService::create_withdrawal_intent(state.db, payload, user_id, claims.evm_address)
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
    Extension(claims): Extension<AuthClaims>,
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<withdraw_db::WithdrawalIntent>>, AppError> {
    let intents = withdraw_db::get_withdrawal_intents_by_user(state.db, user_id).await?;

    // Verify the authenticated user owns these intents by checking evm_address on the first result.
    // If there are no intents we can safely return an empty list — the user_id may just have none.
    if let Some(first) = intents.first() {
        if !first.evm_address.eq_ignore_ascii_case(&claims.evm_address) {
            return Err(AppError::unauthorised(
                "cannot list withdrawal intents for another user",
            ));
        }
    }

    Ok(Json(intents))
}

/// POST /withdraw-intents/transaction — Generate the signed withdrawal transaction data.
///
/// For Hyperliquid this returns EIP-712 typed data the client must sign and submit
/// to Hyperliquid's /exchange endpoint. The server never holds the user's private key.
pub async fn create_withdraw_transaction(
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<CreateWithdrawTransactionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    //todo: fix validation
    // payload.validate()?;

    match payload {
        CreateWithdrawTransactionRequest::Hyperliquid(payload) => {
            let response = hyperliquid::ops::withdraw::withdraw(claims.evm_address, payload.amount)
                .await
                .map_err(|e| AppError::internal(format!("{e:?}")))?;

            let value = serde_json::to_value(response)?;
            Ok(Json(value))
        }
        CreateWithdrawTransactionRequest::Pacifica(payload) => {
            let withdrawal_request = WithdrawRequest {
                account: claims.solana_address,
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
    }
}

pub async fn bridge(
    State(app_state): State<AppState>,
    Extension(claims): Extension<AuthClaims>,
    Json(payload): Json<BridgeRequest>,
) -> Result<Json<QuoteResponse>, AppError> {
    payload.validate()?;

    let validated_origin_chain = Chain::from_id(payload.origin_chain_id);
    let validated_destination_chain = Chain::from_id(payload.destination_chain_id);

    if validated_origin_chain.is_none() || validated_destination_chain.is_none() {
        return Err(AppError::NotFound(String::from(
            "Invalid origin or destination chain id",
        )));
    }

    let validated_origin_chain = validated_origin_chain.unwrap();
    let validated_destination_chain = validated_destination_chain.unwrap();

    let quote_request = QuoteRequest {
        user: claims.evm_address,
        origin_chain_id: validated_origin_chain.id,
        destination_chain_id: validated_destination_chain.id,
        origin_currency: validated_origin_chain.usdc_address.to_string(),
        destination_currency: validated_destination_chain.usdc_address.to_string(),
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
