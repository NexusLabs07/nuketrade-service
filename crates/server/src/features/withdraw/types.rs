// ============================= Request / Response Types =============================

use crate::validation::validate_evm_address;
use db::withdraw::{WithdrawalIntent, WithdrawalStep};
use perp_core::{Chain, exchange::PerpetualExchange};
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn default_destination_chain_id() -> i32 {
    Chain::BASE.id as i32
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateWithdrawalIntentRequest {
    pub exchange: PerpetualExchange,
    #[validate(range(exclusive_min = 0.0, message = "Amount must be greater than 0"))]
    pub amount_usd: f64,
    #[validate(custom(function = "validate_evm_address"))]
    pub recipient: String,
    /// Chain ID of the destination (defaults to Base = 8453).
    #[serde(default = "default_destination_chain_id")]
    pub destination_chain_id: i32,
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
    pub intent: WithdrawalIntent,
    pub steps: Vec<WithdrawalStep>,
}

// ============================= Withdraw Transaction =============================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HyperliquidTransactionRequest {
    /// Amount in USDC as a decimal string (e.g. "100.5").
    #[validate(length(min = 1, message = "Amount must not be empty"))]
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PacificaTransactionRequest {
    pub signature: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateWithdrawTransactionRequest {
    Hyperliquid(HyperliquidTransactionRequest),
    Pacifica(PacificaTransactionRequest),
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct BridgeRequest {
    #[serde(rename = "originChainId")]
    pub origin_chain_id: u64,
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
    #[validate(length(min = 1, message = "amount must not be empty"))]
    pub amount: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "usePermit")]
    pub use_permit: bool,
    #[validate(custom(function = "validate_evm_address"))]
    pub recipient: String,
}

pub struct WithdrawService;
