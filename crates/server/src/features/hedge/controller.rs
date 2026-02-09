use axum::{
    Json,
    extract::{Path, State},
};
use perp_core::exchange::PerpetualExchange;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::user::{validate_evm_address, validate_solana_address};
use crate::services::hedge::NextActionResponse;
use crate::state::AppState;
use crate::{error::AppError, features::hedge::services::HedgeService};
use db::hedge::{self as hedge_db};

// ============================= Request / Response Types =============================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateHedgeIntentRequest {
    pub user_id: uuid::Uuid,
    #[validate(length(min = 1, message = "Asset must not be empty"))]
    pub asset: String,
    pub exchanges: [PerpetualExchange; 2],
    #[validate(range(exclusive_min = 0.0, message = "Margin must be greater than 0"))]
    pub margin_usd: f64,
    #[validate(range(exclusive_min = 0.0, message = "Leverage must be greater than 0"))]
    pub leverage: f64,
    #[validate(custom(function = "validate_evm_address"))]
    pub evm_address: String,
    #[validate(custom(function = "validate_solana_address"))]
    pub solana_address: String,
}

#[derive(Debug, Serialize)]
pub struct CreateHedgeIntentResponse {
    pub hedge_intent_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ActionResultRequest {
    #[validate(length(min = 1, message = "Action must not be empty"))]
    pub action: String,
    #[serde(default)]
    pub success: bool,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
    /// For OPEN_HEDGE_POSITION: per-leg results.
    pub leg_results: Option<Vec<LegResultEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegResultEntry {
    pub exchange: String,
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
pub struct HedgeIntentDetailResponse {
    pub intent: hedge_db::HedgeIntent,
    pub legs: Vec<hedge_db::HedgeLeg>,
}

// ============================= Handlers =============================
/// POST /hedge-intents — Create a new hedge intent with two legs.
pub async fn create_hedge_intent(
    State(state): State<AppState>,
    Json(payload): Json<CreateHedgeIntentRequest>,
) -> Result<Json<CreateHedgeIntentResponse>, AppError> {
    payload.validate()?;

    HedgeService::create_hedge_intent(state.db.clone(), payload)
        .await
        .map(|id| {
            Json(CreateHedgeIntentResponse {
                hedge_intent_id: id,
            })
        })
}

/// GET /hedge-intents/:id/next-action — Compute and return the next executable action.
pub async fn get_next_action(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<NextActionResponse>, AppError> {
    HedgeService::get_next_action(state.db.clone(), state.config, intent_id)
        .await
        .map(Json)
}

/// Query existing balances for each leg and advance legs that don't need
/// bridge and/or deposit. This is called exactly once per intent (on CREATED).
/// POST /hedge-intents/:id/action-result — Client reports the outcome of an executed action.
pub async fn report_action_result(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
    Json(payload): Json<ActionResultRequest>,
) -> Result<Json<ActionResultResponse>, AppError> {
    payload.validate()?;

    HedgeService::report_action_result(state.db, intent_id, payload)
        .await
        .map(|action| {
            Json(ActionResultResponse {
                status: "accepted".to_string(),
                message: format!("Action result for {action} processed"),
            })
        })
}

/// GET /hedge-intents/:id — Get the full intent + legs detail.
pub async fn get_hedge_intent_detail(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<HedgeIntentDetailResponse>, AppError> {
    let intent = hedge_db::get_hedge_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Hedge intent {intent_id}")))?;

    let legs = hedge_db::get_hedge_legs(state.db.clone(), intent_id).await?;

    Ok(Json(HedgeIntentDetailResponse { intent, legs }))
}

/// GET /hedge-intents/user/:user_id — List all intents for a user.
pub async fn list_user_hedge_intents(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<Vec<hedge_db::HedgeIntent>>, AppError> {
    let intents = hedge_db::get_hedge_intents_by_user(state.db.clone(), user_id).await?;
    Ok(Json(intents))
}
