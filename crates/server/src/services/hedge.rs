//! Hedge Intent State Machine
//!
//! Pure logic for computing the next action a client should execute
//! based on the current state of a hedge intent and its legs.
//! This module NEVER touches the database — the controller handles persistence.

use db::hedge::{HedgeIntent, HedgeLeg};
use perp_core::Chain;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Maximum retries per leg action before marking as FAILED.
pub const MAX_RETRIES: i16 = 3;

// ============================= Status Constants =============================

pub mod intent_status {
    pub const CREATED: &str = "CREATED";
    pub const FUNDING: &str = "FUNDING";
    pub const READY: &str = "READY";
    pub const OPENING: &str = "OPENING";
    pub const ACTIVE: &str = "ACTIVE";
    pub const FAILED: &str = "FAILED";
    pub const CANCELLING: &str = "CANCELLING";
    pub const CANCELLED: &str = "CANCELLED";
}

pub mod leg_status {
    pub const PENDING: &str = "PENDING";
    pub const BRIDGE_IN_PROGRESS: &str = "BRIDGE_IN_PROGRESS";
    pub const BRIDGE_CONFIRMED: &str = "BRIDGE_CONFIRMED";
    pub const DEPOSIT_IN_PROGRESS: &str = "DEPOSIT_IN_PROGRESS";
    pub const FUNDED: &str = "FUNDED";
    pub const OPENING_POSITION: &str = "OPENING_POSITION";
    pub const ACTIVE: &str = "ACTIVE";
    pub const FAILED: &str = "FAILED";
    pub const CLOSING: &str = "CLOSING";
    pub const CLOSED: &str = "CLOSED";
}

pub mod action {
    pub const BRIDGE_BASE_TO_ARB: &str = "BRIDGE_BASE_TO_ARB";
    pub const BRIDGE_BASE_TO_SOL: &str = "BRIDGE_BASE_TO_SOL";
    pub const DEPOSIT_TO_HL: &str = "DEPOSIT_TO_HL";
    pub const DEPOSIT_TO_PACIFICA: &str = "DEPOSIT_TO_PACIFICA";
    pub const OPEN_HEDGE_POSITION: &str = "OPEN_HEDGE_POSITION";
    pub const CLOSE_POSITION: &str = "CLOSE_POSITION";
    pub const WAIT: &str = "WAIT";
    pub const NOOP: &str = "NOOP";
}

pub mod protocol {
    pub const HL: &str = "HL";
    pub const PACIFICA: &str = "PACIFICA";
}

pub mod chain {
    pub const ARB: &str = "ARB";
    pub const SOL: &str = "SOL";
}

// ============================= Response Types =============================

/// The action the client should execute next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextActionResponse {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl NextActionResponse {
    pub fn wait() -> Self {
        Self {
            action: action::WAIT.to_string(),
            leg: None,
            amount_usd: None,
            params: None,
        }
    }

    pub fn noop() -> Self {
        Self {
            action: action::NOOP.to_string(),
            leg: None,
            amount_usd: None,
            params: None,
        }
    }
}

/// The result of state machine evaluation — includes the action to return
/// plus any state transitions the controller should persist.
#[derive(Debug)]
pub struct StateMachineOutput {
    pub response: NextActionResponse,
    /// If set, the controller should update hedge_intents.status to this value.
    pub intent_status_update: Option<String>,
    /// (leg_id, new_status) pairs the controller should persist.
    pub leg_status_updates: Vec<(uuid::Uuid, String)>,
}

// ============================= State Machine =============================

/// Main entry point: evaluate the current state and compute what should happen next.
pub fn evaluate(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    match intent.status.as_str() {
        intent_status::CREATED => handle_created(intent, legs),
        intent_status::FUNDING => handle_funding(intent, legs),
        intent_status::READY => handle_ready(intent, legs),
        intent_status::OPENING => handle_opening(intent, legs),
        intent_status::ACTIVE => StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: None,
            leg_status_updates: vec![],
        },
        intent_status::FAILED => handle_failed(intent, legs),
        intent_status::CANCELLING | intent_status::CANCELLED => StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: None,
            leg_status_updates: vec![],
        },
        _ => StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: None,
            leg_status_updates: vec![],
        },
    }
}

// ─── CREATED ──────────────────────────────────────────────────────────────────

fn handle_created(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    // Auto-transition to FUNDING and return the first bridge action.
    let mut output = handle_funding(intent, legs);
    output.intent_status_update = Some(intent_status::FUNDING.to_string());
    output
}

// ─── FUNDING ──────────────────────────────────────────────────────────────────

fn handle_funding(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    // Check if any leg has permanently failed (exceeded max retries).
    let any_permanently_failed = legs
        .iter()
        .any(|l| l.status == leg_status::FAILED && l.retry_count >= MAX_RETRIES);

    if any_permanently_failed {
        return StateMachineOutput {
            response: NextActionResponse {
                action: action::NOOP.to_string(),
                leg: None,
                amount_usd: None,
                params: Some(json!({
                    "error": "One or more legs permanently failed after max retries",
                    "legs": legs.iter().map(|l| json!({
                        "protocol": l.protocol,
                        "status": l.status,
                        "retry_count": l.retry_count,
                        "last_error": l.last_error,
                    })).collect::<Vec<_>>()
                })),
            },
            intent_status_update: Some(intent_status::FAILED.to_string()),
            leg_status_updates: vec![],
        };
    }

    // Check if all legs are funded → transition to READY.
    if legs.iter().all(|l| l.status == leg_status::FUNDED) {
        return handle_all_legs_funded(intent, legs);
    }

    // Find the first leg that needs an action (prioritize legs that aren't in-progress).
    // First pass: legs that can be acted upon immediately.
    for leg in legs {
        match leg.status.as_str() {
            leg_status::PENDING => {
                return StateMachineOutput {
                    response: bridge_action_for(intent, leg),
                    intent_status_update: None,
                    leg_status_updates: vec![],
                };
            }
            leg_status::BRIDGE_CONFIRMED => {
                return StateMachineOutput {
                    response: deposit_action_for(intent, leg),
                    intent_status_update: None,
                    leg_status_updates: vec![],
                };
            }
            leg_status::FAILED if leg.retry_count < MAX_RETRIES => {
                return StateMachineOutput {
                    response: retry_action_for(intent, leg),
                    intent_status_update: None,
                    leg_status_updates: vec![],
                };
            }
            _ => continue, // BRIDGE_IN_PROGRESS, DEPOSIT_IN_PROGRESS, FUNDED — skip
        }
    }

    // All actionable legs are in-progress or funded → WAIT.
    StateMachineOutput {
        response: NextActionResponse::wait(),
        intent_status_update: None,
        leg_status_updates: vec![],
    }
}

fn handle_all_legs_funded(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    let effective_amount = compute_effective_amount(legs);

    StateMachineOutput {
        response: NextActionResponse {
            action: action::OPEN_HEDGE_POSITION.to_string(),
            leg: None,
            amount_usd: Some(effective_amount),
            params: Some(json!({
                "asset": intent.asset,
                "leverage": intent.leverage,
                "effective_margin_usd": effective_amount,
                "legs": legs.iter().map(|l| json!({
                    "protocol": l.protocol,
                    "chain": l.chain,
                    "funded_amount_usd": l.funded_amount_usd,
                })).collect::<Vec<_>>()
            })),
        },
        intent_status_update: Some(intent_status::READY.to_string()),
        leg_status_updates: vec![],
    }
}

// ─── READY ────────────────────────────────────────────────────────────────────

fn handle_ready(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    let effective_amount = compute_effective_amount(legs);

    StateMachineOutput {
        response: NextActionResponse {
            action: action::OPEN_HEDGE_POSITION.to_string(),
            leg: None,
            amount_usd: Some(effective_amount),
            params: Some(json!({
                "asset": intent.asset,
                "leverage": intent.leverage,
                "effective_margin_usd": effective_amount,
                "legs": legs.iter().map(|l| json!({
                    "protocol": l.protocol,
                    "chain": l.chain,
                    "funded_amount_usd": l.funded_amount_usd,
                })).collect::<Vec<_>>()
            })),
        },
        // Transition to OPENING when the client picks up this action.
        intent_status_update: Some(intent_status::OPENING.to_string()),
        leg_status_updates: legs
            .iter()
            .map(|l| (l.id, leg_status::OPENING_POSITION.to_string()))
            .collect(),
    }
}

// ─── OPENING ──────────────────────────────────────────────────────────────────

fn handle_opening(_intent: &HedgeIntent, _legs: &[HedgeLeg]) -> StateMachineOutput {
    // Positions are being opened — client should wait for confirmations
    // (the action-result handler moves state forward).
    StateMachineOutput {
        response: NextActionResponse::wait(),
        intent_status_update: None,
        leg_status_updates: vec![],
    }
}

// ─── FAILED (Safety Mode) ─────────────────────────────────────────────────────

fn handle_failed(intent: &HedgeIntent, legs: &[HedgeLeg]) -> StateMachineOutput {
    // Safety mode: if one leg has an active position and the other failed/closed,
    // we need to close the active leg to prevent directional exposure.
    let needs_closing: Vec<&HedgeLeg> = legs
        .iter()
        .filter(|l| l.status == leg_status::ACTIVE || l.status == leg_status::OPENING_POSITION)
        .collect();

    if !needs_closing.is_empty() {
        // Return CLOSE_POSITION for the first active leg.
        let leg_to_close = needs_closing[0];
        return StateMachineOutput {
            response: NextActionResponse {
                action: action::CLOSE_POSITION.to_string(),
                leg: Some(leg_to_close.protocol.clone()),
                amount_usd: Some(leg_to_close.funded_amount_usd),
                params: Some(json!({
                    "asset": intent.asset,
                    "protocol": leg_to_close.protocol,
                    "chain": leg_to_close.chain,
                    "reason": "safety_mode_partial_hedge",
                })),
            },
            intent_status_update: None,
            leg_status_updates: vec![(leg_to_close.id, leg_status::CLOSING.to_string())],
        };
    }

    // All legs are closed/failed — can transition to CANCELLED.
    let all_resolved = legs.iter().all(|l| {
        l.status == leg_status::FAILED
            || l.status == leg_status::CLOSED
            || l.status == leg_status::PENDING
    });

    if all_resolved {
        return StateMachineOutput {
            response: NextActionResponse::noop(),
            intent_status_update: Some(intent_status::CANCELLED.to_string()),
            leg_status_updates: vec![],
        };
    }

    StateMachineOutput {
        response: NextActionResponse::noop(),
        intent_status_update: None,
        leg_status_updates: vec![],
    }
}

// ============================= Action Builders =============================

fn bridge_action_for(intent: &HedgeIntent, leg: &HedgeLeg) -> NextActionResponse {
    let bridge_amount = compute_bridge_needed(leg);

    let (action_name, origin_chain_id, dest_chain_id, dest_usdc, user_address) =
        match leg.protocol.as_str() {
            protocol::HL => (
                action::BRIDGE_BASE_TO_ARB,
                Chain::BASE.id,
                Chain::ARBITRUM.id,
                Chain::ARBITRUM.usdc_address,
                &intent.evm_address,
            ),
            protocol::PACIFICA => (
                action::BRIDGE_BASE_TO_SOL,
                Chain::BASE.id,
                Chain::SOLANA.id,
                Chain::SOLANA.usdc_address,
                &intent.solana_address,
            ),
            _ => return NextActionResponse::noop(),
        };

    NextActionResponse {
        action: action_name.to_string(),
        leg: Some(leg.protocol.clone()),
        amount_usd: Some(bridge_amount),
        params: Some(json!({
            "origin_chain_id": origin_chain_id,
            "destination_chain_id": dest_chain_id,
            "origin_currency": Chain::BASE.usdc_address,
            "destination_currency": dest_usdc,
            "user_address": user_address,
            "recipient": user_address,
            "leg_id": leg.id.to_string(),
            "existing_margin_usd": leg.existing_margin_usd,
            "existing_onchain_usd": leg.existing_onchain_usd,
        })),
    }
}

fn deposit_action_for(intent: &HedgeIntent, leg: &HedgeLeg) -> NextActionResponse {
    let deposit_amount = compute_deposit_needed(leg);

    let (action_name, user_address) = match leg.protocol.as_str() {
        protocol::HL => (action::DEPOSIT_TO_HL, &intent.evm_address),
        protocol::PACIFICA => (action::DEPOSIT_TO_PACIFICA, &intent.solana_address),
        _ => return NextActionResponse::noop(),
    };

    NextActionResponse {
        action: action_name.to_string(),
        leg: Some(leg.protocol.clone()),
        amount_usd: Some(deposit_amount),
        params: Some(json!({
            "protocol": leg.protocol,
            "chain": leg.chain,
            "user_address": user_address,
            "amount_usd": deposit_amount,
            "leg_id": leg.id.to_string(),
            "existing_margin_usd": leg.existing_margin_usd,
        })),
    }
}

/// For a FAILED leg that still has retries left, figure out which action to retry.
fn retry_action_for(intent: &HedgeIntent, leg: &HedgeLeg) -> NextActionResponse {
    // The leg failed — we need to figure out *what* failed.
    // If it has no successful bridge tx → retry bridge.
    // If it has a bridge but no deposit → retry deposit.
    // We infer this from the last known good status before failure.
    // For simplicity, we look at funded_amount_usd:
    //   - 0 means bridge never completed → retry bridge
    //   - > 0 means bridge completed but deposit failed → retry deposit
    // This is a heuristic; a more robust approach would check tx_references.

    if leg.funded_amount_usd > 0.0 {
        deposit_action_for(intent, leg)
    } else {
        bridge_action_for(intent, leg)
    }
}

// ============================= Helpers =============================

/// Compute the effective margin amount as min(funded_amount) across both legs.
/// This ensures the hedge is delta-neutral.
fn compute_effective_amount(legs: &[HedgeLeg]) -> f64 {
    legs.iter()
        .map(|l| l.funded_amount_usd)
        .fold(f64::MAX, f64::min)
}

/// Map a protocol string to its chain.
pub fn protocol_to_chain(protocol: &str) -> &'static str {
    match protocol {
        protocol::HL => chain::ARB,
        protocol::PACIFICA => chain::SOL,
        _ => "UNKNOWN",
    }
}

/// Map a protocol to the bridge action name.
pub fn protocol_to_bridge_action(protocol: &str) -> &'static str {
    match protocol {
        protocol::HL => action::BRIDGE_BASE_TO_ARB,
        protocol::PACIFICA => action::BRIDGE_BASE_TO_SOL,
        _ => action::NOOP,
    }
}

/// Map a protocol to the deposit action name.
pub fn protocol_to_deposit_action(protocol: &str) -> &'static str {
    match protocol {
        protocol::HL => action::DEPOSIT_TO_HL,
        protocol::PACIFICA => action::DEPOSIT_TO_PACIFICA,
        _ => action::NOOP,
    }
}

/// Compute how much USDC needs to be deposited into the protocol margin.
/// deposit_needed = max(0, target - existing_margin)
pub fn compute_deposit_needed(leg: &HedgeLeg) -> f64 {
    (leg.target_amount_usd - leg.existing_margin_usd).max(0.0)
}

/// Compute how much USDC needs to be bridged from Base.
/// bridge_needed = max(0, deposit_needed - existing_onchain)
pub fn compute_bridge_needed(leg: &HedgeLeg) -> f64 {
    let deposit_needed = compute_deposit_needed(leg);
    (deposit_needed - leg.existing_onchain_usd).max(0.0)
}
