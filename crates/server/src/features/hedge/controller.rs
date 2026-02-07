use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};


use crate::error::AppError;
use crate::services::hedge::{
    self, NextActionResponse, action, intent_status, leg_status, protocol, MAX_RETRIES,
};
use crate::state::AppState;
use db::hedge::{self as hedge_db, NewHedgeIntent, NewHedgeLeg, NewTxReference};

// ============================= Request / Response Types =============================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHedgeIntentRequest {
    pub user_id: uuid::Uuid,
    pub asset: String,
    pub protocols: Vec<String>,
    pub margin_usd: f64,
    pub leverage: f64,
    pub evm_address: String,
    pub solana_address: String,
}

#[derive(Debug, Serialize)]
pub struct CreateHedgeIntentResponse {
    pub hedge_intent_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResultRequest {
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
    pub protocol: String,
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
    // Validate protocols
    if payload.protocols.len() != 2 {
        return Err(AppError::parse("protocols", "Exactly 2 protocols are required"));
    }

    let protocol_a = &payload.protocols[0];
    let protocol_b = &payload.protocols[1];

    // Validate that protocols are supported
    for p in &payload.protocols {
        match p.as_str() {
            protocol::HL | protocol::PACIFICA => {}
            _ => {
                return Err(AppError::parse(
                    "protocols",
                    format!("Unsupported protocol: {}. Supported: HL, PACIFICA", p),
                ));
            }
        }
    }

    if protocol_a == protocol_b {
        return Err(AppError::parse(
            "protocols",
            "Protocols must be different",
        ));
    }

    // Validate margin
    if payload.margin_usd <= 0.0 {
        return Err(AppError::parse("margin_usd", "Margin must be positive"));
    }

    // Validate leverage
    if payload.leverage < 1.0 {
        return Err(AppError::parse("leverage", "Leverage must be >= 1"));
    }

    let intent_id = uuid::Uuid::new_v4();
    let half_margin = payload.margin_usd / 2.0;

    let intent = NewHedgeIntent {
        id: intent_id,
        user_id: payload.user_id,
        asset: payload.asset.clone(),
        protocol_a: protocol_a.clone(),
        protocol_b: protocol_b.clone(),
        margin_usd: payload.margin_usd,
        leverage: payload.leverage,
        evm_address: payload.evm_address.clone(),
        solana_address: payload.solana_address.clone(),
    };

    let legs = vec![
        NewHedgeLeg {
            id: uuid::Uuid::new_v4(),
            hedge_intent_id: intent_id,
            protocol: protocol_a.clone(),
            chain: hedge::protocol_to_chain(protocol_a).to_string(),
            target_amount_usd: half_margin,
        },
        NewHedgeLeg {
            id: uuid::Uuid::new_v4(),
            hedge_intent_id: intent_id,
            protocol: protocol_b.clone(),
            chain: hedge::protocol_to_chain(protocol_b).to_string(),
            target_amount_usd: half_margin,
        },
    ];

    hedge_db::create_hedge_intent_with_legs(state.db.clone(), &intent, &legs).await?;

    log::info!("Created hedge intent {} for user {}", intent_id, payload.user_id);

    Ok(Json(CreateHedgeIntentResponse {
        hedge_intent_id: intent_id,
    }))
}

/// GET /hedge-intents/:id/next-action — Compute and return the next executable action.
pub async fn get_next_action(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<NextActionResponse>, AppError> {
    let intent = hedge_db::get_hedge_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Hedge intent {}", intent_id)))?;

    let legs = hedge_db::get_hedge_legs(state.db.clone(), intent_id).await?;

    // Run the state machine.
    let output = hedge::evaluate(&intent, &legs);

    // Apply any state transitions the state machine computed.
    if let Some(ref new_status) = output.intent_status_update {
        hedge_db::update_hedge_intent_status(state.db.clone(), intent_id, new_status).await?;
        log::info!(
            "Hedge intent {} status: {} → {}",
            intent_id,
            intent.status,
            new_status
        );
    }

    for (leg_id, new_status) in &output.leg_status_updates {
        hedge_db::update_hedge_leg_status(state.db.clone(), *leg_id, new_status).await?;
        log::info!("Hedge leg {} status → {}", leg_id, new_status);
    }

    Ok(Json(output.response))
}

/// POST /hedge-intents/:id/action-result — Client reports the outcome of an executed action.
pub async fn report_action_result(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
    Json(payload): Json<ActionResultRequest>,
) -> Result<Json<ActionResultResponse>, AppError> {
    let intent = hedge_db::get_hedge_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Hedge intent {}", intent_id)))?;

    let legs = hedge_db::get_hedge_legs(state.db.clone(), intent_id).await?;

    match payload.action.as_str() {
        action::BRIDGE_BASE_TO_ARB | action::BRIDGE_BASE_TO_SOL => {
            handle_bridge_result(&state, &intent, &legs, &payload).await?;
        }
        action::DEPOSIT_TO_HL | action::DEPOSIT_TO_PACIFICA => {
            handle_deposit_result(&state, &intent, &legs, &payload).await?;
        }
        action::OPEN_HEDGE_POSITION => {
            handle_open_position_result(&state, &intent, &legs, &payload).await?;
        }
        action::CLOSE_POSITION => {
            handle_close_position_result(&state, &intent, &legs, &payload).await?;
        }
        _ => {
            return Err(AppError::parse(
                "action",
                format!("Unknown action: {}", payload.action),
            ));
        }
    }

    Ok(Json(ActionResultResponse {
        status: "accepted".to_string(),
        message: format!("Action result for {} processed", payload.action),
    }))
}

/// GET /hedge-intents/:id — Get the full intent + legs detail.
pub async fn get_hedge_intent_detail(
    State(state): State<AppState>,
    Path(intent_id): Path<uuid::Uuid>,
) -> Result<Json<HedgeIntentDetailResponse>, AppError> {
    let intent = hedge_db::get_hedge_intent(state.db.clone(), intent_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Hedge intent {}", intent_id)))?;

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

// ============================= Action Result Handlers =============================

/// Handle bridge action result (BRIDGE_BASE_TO_ARB / BRIDGE_BASE_TO_SOL).
async fn handle_bridge_result(
    state: &AppState,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let target_protocol = match payload.action.as_str() {
        action::BRIDGE_BASE_TO_ARB => protocol::HL,
        action::BRIDGE_BASE_TO_SOL => protocol::PACIFICA,
        _ => return Err(AppError::internal("Invalid bridge action")),
    };

    let leg = legs
        .iter()
        .find(|l| l.protocol == target_protocol)
        .ok_or_else(|| AppError::not_found(format!("Leg for protocol {}", target_protocol)))?;

    if payload.success {
        // Record tx reference.
        let tx_ref = NewTxReference {
            id: uuid::Uuid::new_v4(),
            hedge_leg_id: leg.id,
            action: payload.action.clone(),
            tx_hash: payload.tx_hash.clone(),
            chain: leg.chain.clone(),
            status: "CONFIRMED".to_string(),
        };
        hedge_db::insert_tx_reference(state.db.clone(), &tx_ref).await?;

        // Advance leg status.
        hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::BRIDGE_CONFIRMED)
            .await?;

        log::info!(
            "Bridge confirmed for leg {} ({}), tx: {:?}",
            leg.id,
            leg.protocol,
            payload.tx_hash
        );
    } else {
        // Record failure.
        let new_retry_count =
            hedge_db::increment_hedge_leg_retry(state.db.clone(), leg.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "Bridge failed for leg {} ({}), retry {}/{}: {:?}",
            leg.id,
            leg.protocol,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::FAILED)
                .await?;
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::FAILED,
            )
            .await?;
            log::error!(
                "Leg {} ({}) permanently failed after {} retries",
                leg.id,
                leg.protocol,
                MAX_RETRIES
            );
        }
    }

    Ok(())
}

/// Handle deposit action result (DEPOSIT_TO_HL / DEPOSIT_TO_PACIFICA).
async fn handle_deposit_result(
    state: &AppState,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let target_protocol = match payload.action.as_str() {
        action::DEPOSIT_TO_HL => protocol::HL,
        action::DEPOSIT_TO_PACIFICA => protocol::PACIFICA,
        _ => return Err(AppError::internal("Invalid deposit action")),
    };

    let leg = legs
        .iter()
        .find(|l| l.protocol == target_protocol)
        .ok_or_else(|| AppError::not_found(format!("Leg for protocol {}", target_protocol)))?;

    if payload.success {
        // Record tx reference.
        let tx_ref = NewTxReference {
            id: uuid::Uuid::new_v4(),
            hedge_leg_id: leg.id,
            action: payload.action.clone(),
            tx_hash: payload.tx_hash.clone(),
            chain: leg.chain.clone(),
            status: "CONFIRMED".to_string(),
        };
        hedge_db::insert_tx_reference(state.db.clone(), &tx_ref).await?;

        // Mark leg as funded with the target amount.
        hedge_db::update_hedge_leg_funded_amount(state.db.clone(), leg.id, leg.target_amount_usd)
            .await?;
        hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::FUNDED).await?;

        log::info!(
            "Deposit confirmed for leg {} ({}), amount: {}, tx: {:?}",
            leg.id,
            leg.protocol,
            leg.target_amount_usd,
            payload.tx_hash
        );

        // Check if both legs are now funded → auto-transition intent to READY.
        let all_legs = hedge_db::get_hedge_legs(state.db.clone(), intent.id).await?;
        if all_legs.iter().all(|l| l.status == leg_status::FUNDED || l.id == leg.id) {
            // The current leg just became FUNDED (DB not yet reflecting it in `all_legs`
            // since we updated above), so check the other legs.
            let other_legs_funded = all_legs
                .iter()
                .filter(|l| l.id != leg.id)
                .all(|l| l.status == leg_status::FUNDED);

            if other_legs_funded {
                hedge_db::update_hedge_intent_status(
                    state.db.clone(),
                    intent.id,
                    intent_status::READY,
                )
                .await?;
                log::info!("All legs funded — hedge intent {} is READY", intent.id);
            }
        }
    } else {
        let new_retry_count =
            hedge_db::increment_hedge_leg_retry(state.db.clone(), leg.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "Deposit failed for leg {} ({}), retry {}/{}: {:?}",
            leg.id,
            leg.protocol,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::FAILED)
                .await?;
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::FAILED,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle open hedge position result — supports per-leg outcomes for safety mode.
async fn handle_open_position_result(
    state: &AppState,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    // If per-leg results are provided, process them individually.
    if let Some(ref leg_results) = payload.leg_results {
        let mut any_failed = false;
        let mut any_succeeded = false;

        for lr in leg_results {
            let leg = legs
                .iter()
                .find(|l| l.protocol == lr.protocol)
                .ok_or_else(|| AppError::not_found(format!("Leg for protocol {}", lr.protocol)))?;

            if lr.success {
                any_succeeded = true;

                let tx_ref = NewTxReference {
                    id: uuid::Uuid::new_v4(),
                    hedge_leg_id: leg.id,
                    action: action::OPEN_HEDGE_POSITION.to_string(),
                    tx_hash: lr.tx_hash.clone(),
                    chain: leg.chain.clone(),
                    status: "CONFIRMED".to_string(),
                };
                hedge_db::insert_tx_reference(state.db.clone(), &tx_ref).await?;
                hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::ACTIVE)
                    .await?;

                log::info!(
                    "Position opened on {} for leg {}, tx: {:?}",
                    leg.protocol,
                    leg.id,
                    lr.tx_hash
                );
            } else {
                any_failed = true;

                hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::FAILED)
                    .await?;

                log::error!(
                    "Position FAILED on {} for leg {}: {:?}",
                    leg.protocol,
                    leg.id,
                    lr.error
                );
            }
        }

        if any_failed && any_succeeded {
            // SAFETY MODE: partial execution — need to close the opened leg.
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::FAILED,
            )
            .await?;
            log::error!(
                "SAFETY MODE ACTIVATED for intent {}: partial position open detected",
                intent.id
            );
        } else if any_failed {
            // Both failed.
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::FAILED,
            )
            .await?;
        } else {
            // Both succeeded — hedge is live!
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::ACTIVE,
            )
            .await?;
            log::info!("Hedge intent {} is now ACTIVE", intent.id);
        }

        return Ok(());
    }

    // Fallback: no per-leg results, use top-level success/failure.
    if payload.success {
        for leg in legs {
            if let Some(ref tx) = payload.tx_hash {
                let tx_ref = NewTxReference {
                    id: uuid::Uuid::new_v4(),
                    hedge_leg_id: leg.id,
                    action: action::OPEN_HEDGE_POSITION.to_string(),
                    tx_hash: Some(tx.clone()),
                    chain: leg.chain.clone(),
                    status: "CONFIRMED".to_string(),
                };
                hedge_db::insert_tx_reference(state.db.clone(), &tx_ref).await?;
            }
            hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::ACTIVE)
                .await?;
        }
        hedge_db::update_hedge_intent_status(state.db.clone(), intent.id, intent_status::ACTIVE)
            .await?;
        log::info!("Hedge intent {} is now ACTIVE", intent.id);
    } else {
        for leg in legs {
            hedge_db::update_hedge_leg_status(state.db.clone(), leg.id, leg_status::FAILED)
                .await?;
        }
        hedge_db::update_hedge_intent_status(state.db.clone(), intent.id, intent_status::FAILED)
            .await?;
        log::error!(
            "Hedge position open failed for intent {}: {:?}",
            intent.id,
            payload.error
        );
    }

    Ok(())
}

/// Handle close position result (safety mode unwinding).
async fn handle_close_position_result(
    state: &AppState,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    // Find the leg being closed (the one in CLOSING state).
    let closing_leg = legs
        .iter()
        .find(|l| l.status == leg_status::CLOSING || l.status == leg_status::ACTIVE)
        .ok_or_else(|| AppError::not_found("Leg in CLOSING state"))?;

    if payload.success {
        let tx_ref = NewTxReference {
            id: uuid::Uuid::new_v4(),
            hedge_leg_id: closing_leg.id,
            action: action::CLOSE_POSITION.to_string(),
            tx_hash: payload.tx_hash.clone(),
            chain: closing_leg.chain.clone(),
            status: "CONFIRMED".to_string(),
        };
        hedge_db::insert_tx_reference(state.db.clone(), &tx_ref).await?;
        hedge_db::update_hedge_leg_status(state.db.clone(), closing_leg.id, leg_status::CLOSED)
            .await?;

        log::info!(
            "Position closed on {} for leg {} (safety mode)",
            closing_leg.protocol,
            closing_leg.id
        );

        // Check if all legs are now resolved.
        let updated_legs = hedge_db::get_hedge_legs(state.db.clone(), intent.id).await?;
        let all_resolved = updated_legs.iter().all(|l| {
            l.status == leg_status::FAILED
                || l.status == leg_status::CLOSED
                || l.status == leg_status::PENDING
        });

        if all_resolved {
            hedge_db::update_hedge_intent_status(
                state.db.clone(),
                intent.id,
                intent_status::CANCELLED,
            )
            .await?;
            log::info!(
                "Hedge intent {} fully unwound → CANCELLED",
                intent.id
            );
        }
    } else {
        // Close failed — increment retry.
        let new_retry_count = hedge_db::increment_hedge_leg_retry(
            state.db.clone(),
            closing_leg.id,
            payload.error.as_deref(),
        )
        .await?;

        log::error!(
            "Close position failed for leg {} ({}), retry {}/{}: {:?}",
            closing_leg.id,
            closing_leg.protocol,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );
    }

    Ok(())
}
