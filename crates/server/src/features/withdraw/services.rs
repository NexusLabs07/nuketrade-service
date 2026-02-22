use std::sync::Arc;

use db::withdraw::{self as withdraw_db, NewWithdrawalIntent, NewWithdrawalStep};
use perp_core::exchange::{PerpetualExchange, exchange_to_chain};
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    error::AppError,
    features::withdraw::controller::{ActionResultRequest, CreateWithdrawalIntentRequest},
    services::withdraw::{self as withdraw_sm, NextActionResponse, MAX_RETRIES},
};

pub struct WithdrawService;

impl WithdrawService {
    pub async fn create_withdrawal_intent(
        db: Arc<PgPool>,
        payload: CreateWithdrawalIntentRequest,
        user_id: Uuid,
    ) -> Result<Uuid, AppError> {
        if payload.amount_usd <= 0.0 {
            return Err(AppError::parse("amount_usd", "Amount must be greater than 0"));
        }

        let intent_id = Uuid::new_v4();

        let exchange_enum = PerpetualExchange::from_str(&payload.exchange.to_string())
            .map_err(|_| AppError::parse("exchange", "Unknown exchange"))?;

        let exchange_chain_id = exchange_to_chain(&exchange_enum) as i32;

        let new_intent = NewWithdrawalIntent {
            id: intent_id,
            user_id,
            exchange: exchange_enum.to_string(),
            amount_usd: payload.amount_usd,
            evm_address: payload.evm_address.clone(),
            recipient: payload.recipient.clone(),
            destination_chain_id: payload.destination_chain_id,
        };

        withdraw_db::create_withdrawal_intent(db.clone(), &new_intent).await?;

        // Create the initial WITHDRAW step as PENDING upfront for the audit trail.
        let withdraw_step = NewWithdrawalStep {
            id: Uuid::new_v4(),
            withdrawal_intent_id: intent_id,
            step: withdraw_sm::step_name::WITHDRAW.to_string(),
            chain_id: Some(exchange_chain_id),
        };
        withdraw_db::insert_withdrawal_step(db.clone(), &withdraw_step).await?;

        log::info!(
            "Created withdrawal intent {} for user {} ({} → Base, {:.2} USD)",
            intent_id,
            user_id,
            payload.exchange,
            payload.amount_usd,
        );

        Ok(intent_id)
    }

    pub async fn get_next_action(
        db: Arc<PgPool>,
        intent_id: Uuid,
    ) -> Result<NextActionResponse, AppError> {
        let intent = withdraw_db::get_withdrawal_intent(db.clone(), intent_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Withdrawal intent {intent_id}")))?;

        let output = match withdraw_sm::evaluate(&intent) {
            Ok(o) => o,
            Err(reason) => {
                log::error!(
                    "Withdrawal intent {} state machine error: {}",
                    intent_id,
                    reason
                );
                withdraw_db::update_withdrawal_intent_status(
                    db.clone(),
                    intent_id,
                    withdraw_sm::intent_status::FAILED,
                )
                .await?;
                return Err(AppError::internal(format!(
                    "withdrawal intent {} is in an unrecoverable state: {}",
                    intent_id, reason
                )));
            }
        };

        if let Some(ref new_status) = output.intent_status_update {
            withdraw_db::update_withdrawal_intent_status(db.clone(), intent_id, new_status).await?;
            log::info!(
                "Withdrawal intent {} status: {} → {}",
                intent_id,
                intent.status,
                new_status
            );
        }

        Ok(output.response)
    }

    pub async fn report_action_result(
        db: Arc<PgPool>,
        intent_id: Uuid,
        payload: ActionResultRequest,
    ) -> Result<String, AppError> {
        let intent = withdraw_db::get_withdrawal_intent(db.clone(), intent_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Withdrawal intent {intent_id}")))?;

        match payload.action.as_str() {
            withdraw_sm::action::WITHDRAW => {
                handle_withdraw_result(db, &intent, &payload).await?;
            }
            withdraw_sm::action::BRIDGE => {
                handle_bridge_result(db, &intent, &payload).await?;
            }
            _ => {
                return Err(AppError::parse(
                    "action",
                    format!("Unknown action: {}", payload.action),
                ));
            }
        }

        Ok(payload.action)
    }
}

// ============================= Action Result Handlers =============================

async fn handle_withdraw_result(
    db: Arc<PgPool>,
    intent: &withdraw_db::WithdrawalIntent,
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let step = withdraw_db::get_pending_step(
        db.clone(),
        intent.id,
        withdraw_sm::step_name::WITHDRAW,
    )
    .await?
    .ok_or_else(|| AppError::not_found("Pending WITHDRAW step"))?;

    if payload.success {
        withdraw_db::update_withdrawal_step_status(
            db.clone(),
            step.id,
            payload.tx_hash.as_deref(),
            withdraw_sm::step_status::CONFIRMED,
        )
        .await?;

        // Create the BRIDGE step now that USDC is on the protocol chain.
        let exchange_enum = PerpetualExchange::from_str(&intent.exchange).ok();
        let bridge_chain_id = exchange_enum
            .as_ref()
            .and_then(|e| e.chain())
            .map(|c| c.id as i32);

        let bridge_step = NewWithdrawalStep {
            id: Uuid::new_v4(),
            withdrawal_intent_id: intent.id,
            step: withdraw_sm::step_name::BRIDGE.to_string(),
            chain_id: bridge_chain_id,
        };
        withdraw_db::insert_withdrawal_step(db.clone(), &bridge_step).await?;

        withdraw_db::update_withdrawal_intent_status(
            db.clone(),
            intent.id,
            withdraw_sm::intent_status::WITHDRAWN,
        )
        .await?;

        log::info!(
            "Withdrawal step confirmed for intent {}, tx: {:?}",
            intent.id,
            payload.tx_hash
        );
    } else {
        let new_retry_count =
            withdraw_db::increment_withdrawal_intent_retry(db.clone(), intent.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "WITHDRAW failed for intent {}, retry {}/{}: {:?}",
            intent.id,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            withdraw_db::update_withdrawal_step_status(
                db.clone(),
                step.id,
                None,
                withdraw_sm::step_status::FAILED,
            )
            .await?;
            withdraw_db::update_withdrawal_intent_status(
                db.clone(),
                intent.id,
                withdraw_sm::intent_status::FAILED,
            )
            .await?;
            log::error!(
                "Withdrawal intent {} permanently failed after {} retries",
                intent.id,
                MAX_RETRIES
            );
        }
    }

    Ok(())
}

async fn handle_bridge_result(
    db: Arc<PgPool>,
    intent: &withdraw_db::WithdrawalIntent,
    payload: &ActionResultRequest,
) -> Result<(), AppError> {
    let step =
        withdraw_db::get_pending_step(db.clone(), intent.id, withdraw_sm::step_name::BRIDGE)
            .await?
            .ok_or_else(|| AppError::not_found("Pending BRIDGE step"))?;

    if payload.success {
        withdraw_db::update_withdrawal_step_status(
            db.clone(),
            step.id,
            payload.tx_hash.as_deref(),
            withdraw_sm::step_status::CONFIRMED,
        )
        .await?;

        withdraw_db::update_withdrawal_intent_status(
            db.clone(),
            intent.id,
            withdraw_sm::intent_status::COMPLETED,
        )
        .await?;

        log::info!(
            "Withdrawal intent {} COMPLETED. USDC bridged to Base, tx: {:?}",
            intent.id,
            payload.tx_hash
        );
    } else {
        let new_retry_count =
            withdraw_db::increment_withdrawal_intent_retry(db.clone(), intent.id, payload.error.as_deref())
                .await?;

        log::warn!(
            "BRIDGE failed for intent {}, retry {}/{}: {:?}",
            intent.id,
            new_retry_count,
            MAX_RETRIES,
            payload.error
        );

        if new_retry_count >= MAX_RETRIES {
            withdraw_db::update_withdrawal_step_status(
                db.clone(),
                step.id,
                None,
                withdraw_sm::step_status::FAILED,
            )
            .await?;
            withdraw_db::update_withdrawal_intent_status(
                db.clone(),
                intent.id,
                withdraw_sm::intent_status::FAILED,
            )
            .await?;
            log::error!(
                "Withdrawal intent {} permanently failed after {} retries (BRIDGE step)",
                intent.id,
                MAX_RETRIES
            );
        }
    }

    Ok(())
}
