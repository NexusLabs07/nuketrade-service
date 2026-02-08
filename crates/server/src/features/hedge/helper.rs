use std::sync::Arc;

use crate::features::hedge::controller::ActionResultRequest;
use crate::services::balance::compute_funding_needs;
use crate::services::hedge::{MAX_RETRIES, action, intent_status, leg_status};
use crate::{error::AppError, services::balance};
use db::hedge::{self as hedge_db};
use db::hedge::{HedgeLeg, NewTxReference};
use perp_core::config::Config;
use perp_core::exchange::PerpetualExchange;
use sqlx::PgPool;

pub async fn check_and_apply_existing_balances(
    db: Arc<PgPool>,
    config: Config,
    intent: &hedge_db::HedgeIntent,
    legs: &mut [HedgeLeg],
) -> Result<(), AppError> {
    for leg in legs.iter_mut() {
        // Query existing balances for this protocol.
        let balances = balance::check_leg_balances(
            &config,
            &leg.exchange,
            &intent.evm_address,
            &intent.solana_address,
        )
        .await;

        // Persist the raw balance snapshot for auditing.
        hedge_db::update_hedge_leg_existing_balances(
            db.clone(),
            leg.id,
            balances.exchange_margin_used,
            balances.onchain_usd,
        )
        .await?;

        // Update in-memory leg too (so the state machine sees correct values).
        leg.existing_margin_usd = balances.exchange_margin_used;
        leg.existing_onchain_usd = balances.onchain_usd;

        // Compute funding needs.
        let needs = compute_funding_needs(
            leg.target_amount_usd,
            balances.exchange_margin_used,
            balances.onchain_usd,
        );

        log::info!(
            "Leg {} ({}) balance check: margin={:.2}, onchain={:.2} → deposit_needed={:.2}, bridge_needed={:.2}",
            leg.id,
            leg.exchange,
            balances.exchange_margin_used,
            balances.onchain_usd,
            needs.deposit_needed,
            needs.bridge_needed,
        );

        if needs.deposit_needed <= 0.0 {
            // Protocol margin already has enough — skip bridge AND deposit.
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FUNDED).await?;
            hedge_db::update_hedge_leg_funded_amount(db.clone(), leg.id, leg.target_amount_usd)
                .await?;
            log::info!(
                "Leg {} ({}) already funded from existing margin ({:.2} >= {:.2})",
                leg.id,
                leg.exchange,
                balances.exchange_margin_used,
                leg.target_amount_usd,
            );
        } else if needs.bridge_needed <= 0.0 {
            // On-chain balance covers the deposit need — skip bridge, go straight to deposit.
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::BRIDGE_CONFIRMED)
                .await?;
            log::info!(
                "Leg {} ({}) skipping bridge — on-chain balance ({:.2}) covers deposit need ({:.2})",
                leg.id,
                leg.exchange,
                balances.onchain_usd,
                needs.deposit_needed,
            );
        }
        // else: stays PENDING — needs full bridge + deposit
    }

    Ok(())
}

// ============================= Action Result Handlers =============================

/// Handle bridge action result (BRIDGE_BASE_TO_ARB / BRIDGE_BASE_TO_SOL).
pub async fn handle_bridge_result(
    db: Arc<PgPool>,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let target_exchange = PerpetualExchange::from_bridge_action(&payload.action)
        .ok_or_else(|| AppError::internal("Invalid bridge action"))?;

    let leg = legs
        .iter()
        .find(|l| l.exchange == target_exchange.to_string())
        .ok_or_else(|| AppError::not_found(format!("Leg for exchange {target_exchange}")))?;

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
        hedge_db::insert_tx_reference(db.clone(), &tx_ref).await?;

        // Advance leg status.
        hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::BRIDGE_CONFIRMED).await?;

        log::info!(
            "Bridge confirmed for leg {} ({}), tx: {:?}",
            leg.id,
            leg.exchange,
            payload.tx_hash
        );
    } else {
        // Record failure.
        let new_retry_count =
            hedge_db::increment_hedge_leg_retry(db.clone(), leg.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "Bridge failed for leg {} ({}), retry {}/{}: {:?}",
            leg.id,
            leg.exchange,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FAILED).await?;
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::FAILED)
                .await?;
            log::error!(
                "Leg {} ({}) permanently failed after {} retries",
                leg.id,
                leg.exchange,
                MAX_RETRIES
            );
        }
    }

    Ok(())
}

/// Handle deposit action result (DEPOSIT_TO_HL / DEPOSIT_TO_PACIFICA).
pub async fn handle_deposit_result(
    db: Arc<PgPool>,
    intent: &hedge_db::HedgeIntent,
    legs: &[hedge_db::HedgeLeg],
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let target_exchange = PerpetualExchange::from_deposit_action(&payload.action)
        .ok_or_else(|| AppError::internal("Invalid deposit action"))?;

    let leg = legs
        .iter()
        .find(|l| l.exchange == target_exchange.to_string())
        .ok_or_else(|| AppError::not_found(format!("Leg for exchange {target_exchange}")))?;

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
        hedge_db::insert_tx_reference(db.clone(), &tx_ref).await?;

        // Mark leg as funded with the target amount.
        hedge_db::update_hedge_leg_funded_amount(db.clone(), leg.id, leg.target_amount_usd).await?;
        hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FUNDED).await?;

        log::info!(
            "Deposit confirmed for leg {} ({}), amount: {}, tx: {:?}",
            leg.id,
            leg.exchange,
            leg.target_amount_usd,
            payload.tx_hash
        );

        // Check if both legs are now funded → auto-transition intent to READY.
        let all_legs = hedge_db::get_hedge_legs(db.clone(), intent.id).await?;
        if all_legs
            .iter()
            .all(|l| l.status == leg_status::FUNDED || l.id == leg.id)
        {
            // The current leg just became FUNDED (DB not yet reflecting it in `all_legs`
            // since we updated above), so check the other legs.
            let other_legs_funded = all_legs
                .iter()
                .filter(|l| l.id != leg.id)
                .all(|l| l.status == leg_status::FUNDED);

            if other_legs_funded {
                hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::READY)
                    .await?;
                log::info!("All legs funded — hedge intent {} is READY", intent.id);
            }
        }
    } else {
        let new_retry_count =
            hedge_db::increment_hedge_leg_retry(db.clone(), leg.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "Deposit failed for leg {} ({}), retry {}/{}: {:?}",
            leg.id,
            leg.exchange,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FAILED).await?;
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::FAILED)
                .await?;
        }
    }

    Ok(())
}

/// Handle open hedge position result — supports per-leg outcomes for safety mode.
pub async fn handle_open_position_result(
    db: Arc<PgPool>,
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
                .find(|l| l.exchange == lr.exchange)
                .ok_or_else(|| AppError::not_found(format!("Leg for exchange {}", lr.exchange)))?;

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
                hedge_db::insert_tx_reference(db.clone(), &tx_ref).await?;
                hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::ACTIVE).await?;

                log::info!(
                    "Position opened on {} for leg {}, tx: {:?}",
                    leg.exchange,
                    leg.id,
                    lr.tx_hash
                );
            } else {
                any_failed = true;

                hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FAILED).await?;

                log::error!(
                    "Position FAILED on {} for leg {}: {:?}",
                    leg.exchange,
                    leg.id,
                    lr.error
                );
            }
        }

        if any_failed && any_succeeded {
            // SAFETY MODE: partial execution — need to close the opened leg.
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::FAILED)
                .await?;
            log::error!(
                "SAFETY MODE ACTIVATED for intent {}: partial position open detected",
                intent.id
            );
        } else if any_failed {
            // Both failed.
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::FAILED)
                .await?;
        } else {
            // Both succeeded — hedge is live!
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::ACTIVE)
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
                hedge_db::insert_tx_reference(db.clone(), &tx_ref).await?;
            }
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::ACTIVE).await?;
        }
        hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::ACTIVE).await?;
        log::info!("Hedge intent {} is now ACTIVE", intent.id);
    } else {
        for leg in legs {
            hedge_db::update_hedge_leg_status(db.clone(), leg.id, leg_status::FAILED).await?;
        }
        hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::FAILED).await?;
        log::error!(
            "Hedge position open failed for intent {}: {:?}",
            intent.id,
            payload.error
        );
    }

    Ok(())
}

/// Handle close position result (safety mode unwinding).
pub async fn handle_close_position_result(
    db: Arc<PgPool>,
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
        hedge_db::insert_tx_reference(db.clone(), &tx_ref).await?;
        hedge_db::update_hedge_leg_status(db.clone(), closing_leg.id, leg_status::CLOSED).await?;

        log::info!(
            "Position closed on {} for leg {} (safety mode)",
            closing_leg.exchange,
            closing_leg.id
        );

        // Check if all legs are now resolved.
        let updated_legs = hedge_db::get_hedge_legs(db.clone(), intent.id).await?;
        let all_resolved = updated_legs.iter().all(|l| {
            l.status == leg_status::FAILED
                || l.status == leg_status::CLOSED
                || l.status == leg_status::PENDING
        });

        if all_resolved {
            hedge_db::update_hedge_intent_status(db.clone(), intent.id, intent_status::CANCELLED)
                .await?;
            log::info!("Hedge intent {} fully unwound → CANCELLED", intent.id);
        }
    } else {
        // Close failed — increment retry.
        let new_retry_count = hedge_db::increment_hedge_leg_retry(
            db.clone(),
            closing_leg.id,
            payload.error.as_deref(),
        )
        .await?;

        log::error!(
            "Close position failed for leg {} ({}), retry {}/{}: {:?}",
            closing_leg.id,
            closing_leg.exchange,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );
    }

    Ok(())
}
