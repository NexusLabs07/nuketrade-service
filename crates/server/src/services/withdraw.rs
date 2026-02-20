//! Withdrawal Intent State Machine
//!
//! Pure logic for computing the next action a client should execute
//! based on the current state of a withdrawal intent.
//! This module NEVER touches the database — the feature service handles persistence.

use db::withdraw::WithdrawalIntent;
use perp_core::{Chain, exchange::PerpetualExchange};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::str::FromStr;

pub const MAX_RETRIES: i16 = 3;

// ============================= Status Constants =============================

pub mod intent_status {
    pub const CREATED: &str = "CREATED";
    pub const WITHDRAWING: &str = "WITHDRAWING";
    pub const WITHDRAWN: &str = "WITHDRAWN";
    pub const BRIDGING: &str = "BRIDGING";
    pub const COMPLETED: &str = "COMPLETED";
    pub const FAILED: &str = "FAILED";
}

pub mod step_name {
    pub const WITHDRAW: &str = "WITHDRAW";
    pub const BRIDGE: &str = "BRIDGE";
}

pub mod step_status {
    pub const CONFIRMED: &str = "CONFIRMED";
    pub const FAILED: &str = "FAILED";
}

pub mod action {
    pub const WITHDRAW: &str = "WITHDRAW";
    pub const BRIDGE: &str = "BRIDGE";
    pub const WAIT: &str = "WAIT";
    pub const NOOP: &str = "NOOP";
}

// ============================= Response Types =============================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextActionResponse {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl NextActionResponse {
    pub fn wait() -> Self {
        Self {
            action: action::WAIT.to_string(),
            params: None,
        }
    }

    pub fn noop() -> Self {
        Self {
            action: action::NOOP.to_string(),
            params: None,
        }
    }
}

/// The result of state machine evaluation — includes the action to return
/// plus any intent status transition the service should persist.
#[derive(Debug)]
pub struct StateMachineOutput {
    pub response: NextActionResponse,
    /// If set, update withdrawal_intents.status to this value.
    pub intent_status_update: Option<String>,
}

// ============================= State Machine =============================

pub fn evaluate(intent: &WithdrawalIntent) -> StateMachineOutput {
    match intent.status.as_str() {
        intent_status::CREATED => handle_created(intent),
        intent_status::WITHDRAWING => StateMachineOutput {
            response: NextActionResponse::wait(),
            intent_status_update: None,
        },
        intent_status::WITHDRAWN => handle_withdrawn(intent),
        intent_status::BRIDGING => StateMachineOutput {
            response: NextActionResponse::wait(),
            intent_status_update: None,
        },
        intent_status::COMPLETED => StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: None,
        },
        intent_status::FAILED => StateMachineOutput {
            response: NextActionResponse {
                action: action::NOOP.to_string(),
                params: Some(json!({
                    "error": intent.last_error,
                    "retry_count": intent.retry_count,
                })),
            },
            intent_status_update: None,
        },
        _ => StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: None,
        },
    }
}

// ─── CREATED ──────────────────────────────────────────────────────────────────

fn handle_created(intent: &WithdrawalIntent) -> StateMachineOutput {
    StateMachineOutput {
        response: NextActionResponse {
            action: action::WITHDRAW.to_string(),
            params: Some(json!({
                "exchange": intent.exchange,
                "amount_usd": intent.amount_usd,
                "evm_address": intent.evm_address,
            })),
        },
        // Advance intent to WITHDRAWING so subsequent polls return WAIT.
        intent_status_update: Some(intent_status::WITHDRAWING.to_string()),
    }
}

// ─── WITHDRAWN ────────────────────────────────────────────────────────────────

fn handle_withdrawn(intent: &WithdrawalIntent) -> StateMachineOutput {
    let exchange = PerpetualExchange::from_str(&intent.exchange).ok();
    let origin_chain = exchange.as_ref().and_then(|e| e.chain());

    let params = if let Some(chain) = origin_chain {
        json!({
            "origin_chain_id": chain.id,
            "destination_chain_id": intent.destination_chain_id,
            "origin_currency": chain.usdc_address,
            "destination_currency": Chain::BASE.usdc_address,
            "amount_usd": intent.amount_usd,
            "recipient": intent.recipient,
            "user_address": intent.evm_address,
        })
    } else {
        // Unknown exchange chain — return what we can; client can infer.
        json!({
            "destination_chain_id": intent.destination_chain_id,
            "destination_currency": Chain::BASE.usdc_address,
            "amount_usd": intent.amount_usd,
            "recipient": intent.recipient,
            "user_address": intent.evm_address,
        })
    };

    StateMachineOutput {
        response: NextActionResponse {
            action: action::BRIDGE.to_string(),
            params: Some(params),
        },
        // Advance to BRIDGING so subsequent polls return WAIT.
        intent_status_update: Some(intent_status::BRIDGING.to_string()),
    }
}
