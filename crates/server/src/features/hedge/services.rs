use std::sync::Arc;

use db::hedge::{self as hedge_db, NewHedgeIntent, NewHedgeLeg};
use perp_core::{
    config::Config,
    exchange::{PerpetualExchange, exchange_to_chain},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    features::hedge::{
        controller::{ActionResultRequest, CreateHedgeIntentRequest},
        helper::{
            check_and_apply_existing_balances, handle_bridge_result, handle_close_position_result,
            handle_deposit_result, handle_open_position_result,
        },
    },
    services::hedge::{self, NextActionResponse, action, intent_status},
};

pub struct HedgeService;

impl Default for HedgeService {
    fn default() -> Self {
        Self::new()
    }
}

impl HedgeService {
    pub fn new() -> Self {
        Self {}
    }

    // Add any shared business logic methods for hedge operations here
    pub async fn create_hedge_intent(
        db: Arc<PgPool>,
        payload: CreateHedgeIntentRequest,
    ) -> Result<Uuid, AppError> {
        let exchange_a = &payload.exchanges[0];
        let exchange_b = &payload.exchanges[1];

        let exchange_a_str = exchange_a.to_string();
        let exchange_b_str = exchange_b.to_string();

        let intent_id = uuid::Uuid::new_v4();
        let half_margin = payload.margin_usd / 2.0;

        let intent = NewHedgeIntent {
            id: intent_id,
            user_id: payload.user_id,
            asset: payload.asset.clone(),
            exchange_a: exchange_a_str.clone(),
            exchange_b: exchange_b_str.clone(),
            margin_usd: payload.margin_usd,
            leverage: payload.leverage,
            evm_address: payload.evm_address.clone(),
            solana_address: payload.solana_address.clone(),
        };

        let legs = vec![
            NewHedgeLeg {
                id: uuid::Uuid::new_v4(),
                hedge_intent_id: intent_id,
                exchange: exchange_a_str.clone(),
                chain: exchange_to_chain(exchange_a) as i32,
                target_amount_usd: half_margin,
            },
            NewHedgeLeg {
                id: uuid::Uuid::new_v4(),
                hedge_intent_id: intent_id,
                exchange: exchange_b_str.clone(),
                chain: exchange_to_chain(exchange_b) as i32,
                target_amount_usd: half_margin,
            },
        ];

        hedge_db::create_hedge_intent_with_legs(db.clone(), &intent, &legs).await?;

        log::info!(
            "Created hedge intent {} for user {}",
            intent_id,
            payload.user_id
        );

        Ok(intent_id)
    }

    pub async fn get_next_action(
        db: Arc<PgPool>,
        config: Config,
        intent_id: Uuid,
    ) -> Result<NextActionResponse, AppError> {
        let intent = hedge_db::get_hedge_intent(db.clone(), intent_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Hedge intent {intent_id}")))?;

        let mut legs = hedge_db::get_hedge_legs(db.clone(), intent_id).await?;

        // ── Balance check on CREATED → FUNDING boundary ─────────────────────
        // When the intent is fresh (CREATED), query existing balances on both
        // protocols/chains and skip bridge/deposit steps for legs that are
        // already funded (partially or fully).
        if intent.status == intent_status::CREATED {
            check_and_apply_existing_balances(db.clone(), config, &intent, &mut legs).await?;
            // Re-fetch legs with updated statuses/balances.
            legs = hedge_db::get_hedge_legs(db.clone(), intent_id).await?;
        }

        // Run the state machine.
        let output = hedge::evaluate(&intent, &legs);

        // Apply any state transitions the state machine computed.
        if let Some(ref new_status) = output.intent_status_update {
            hedge_db::update_hedge_intent_status(db.clone(), intent_id, new_status).await?;
            log::info!(
                "Hedge intent {} status: {} → {}",
                intent_id,
                intent.status,
                new_status
            );
        }

        for (leg_id, new_status) in &output.leg_status_updates {
            hedge_db::update_hedge_leg_status(db.clone(), *leg_id, new_status).await?;
            log::info!("Hedge leg {leg_id} status → {new_status}");
        }

        Ok(output.response)
    }

    pub async fn report_action_result(
        db: Arc<PgPool>,
        intent_id: Uuid,
        payload: ActionResultRequest,
    ) -> Result<String, AppError> {
        let intent = hedge_db::get_hedge_intent(db.clone(), intent_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("Hedge intent {intent_id}")))?;

        let legs = hedge_db::get_hedge_legs(db.clone(), intent_id).await?;

        let action_str = payload.action.as_str();

        if PerpetualExchange::is_bridge_action(action_str) {
            handle_bridge_result(db.clone(), &intent, &legs, &payload).await?;
        } else if PerpetualExchange::is_deposit_action(action_str) {
            handle_deposit_result(db.clone(), &intent, &legs, &payload).await?;
        } else if action_str == action::OPEN_HEDGE_POSITION {
            handle_open_position_result(db.clone(), &intent, &legs, &payload).await?;
        } else if action_str == action::CLOSE_POSITION {
            handle_close_position_result(db.clone(), &intent, &legs, &payload).await?;
        } else {
            return Err(AppError::parse(
                "action",
                format!("Unknown action: {}", payload.action),
            ));
        }

        Ok(payload.action)
    }
}
