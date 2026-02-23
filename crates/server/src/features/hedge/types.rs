// ============================= Request / Response Types =============================

use crate::validation::hedge::validate_distinct_exchanges;
use db::hedge::{HedgeIntent, HedgeLeg};
use perp_core::exchange::PerpetualExchange;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateHedgeIntentRequest {
    #[validate(length(min = 1, message = "Asset must not be empty"))]
    pub asset: String,
    #[validate(custom(function = "validate_distinct_exchanges"))]
    pub exchanges: [PerpetualExchange; 2],
    #[validate(range(exclusive_min = 0.0, message = "Margin must be greater than 0"))]
    pub margin_usd: f64,
    #[validate(range(min = 1.0, message = "Leverage must be >= 1"))]
    pub leverage: f64,
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
    pub intent: HedgeIntent,
    pub legs: Vec<HedgeLeg>,
}

pub struct HedgeService;
