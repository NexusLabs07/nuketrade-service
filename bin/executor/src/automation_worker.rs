//! Automation worker — in-process Rust executor for the planned Node
//! worker referenced in V13 (never built).
//!
//! Flow per tick:
//!   1. `AutomationService::fetch_due_intents` returns intents whose APR
//!      crossed the user's threshold AND which aren't already in position
//!      (the engine checks `current_legs`). Each intent is also leased
//!      to this worker.
//!   2. For each intent we look up the user's Hyperliquid agent wallet
//!      (one per user, stored in `hl_agent_wallets`). HL's agent model
//!      is 1-to-1 with the master account, so a "global" agent wallet
//!      shared across users would not work.
//!   3. Build the HL `Order` action from the intent's HL leg, sign the
//!      EIP-712 digest with the user's agent wallet via Turnkey, POST
//!      to HL.
//!   4. Report the result back via `record_intent_result`. SUCCEEDED on
//!      OPEN causes the engine to write `current_legs` — that's the
//!      "marks opened" marker that prevents re-opening on the next tick.
//!
//! Non-HL legs (e.g. Pacifica) are not executed in v1; intents with no
//! HL leg fail-fast with a clear error.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use db::wallet::queries::get_hl_agent_for_user;
use hyperliquid::{
    helpers::markets::{get_asset_index, get_sz_decimals},
    services::exchange::{
        ExchangeAction, HlClient, HlEnv, HlSignature, LimitWire, OrderRequest, OrderTypeWire,
        SignerBackend,
    },
};
use perp_core::{SevenDayApr, config::Config};
use server::{
    features::automation::services::{AutomationService, DueIntent, IntentResult, IntentResultLeg},
    services::turnkey::TurnkeyClient,
    types::FeedSnapshot,
};
use sqlx::PgPool;
use tokio::sync::watch;

const FETCH_LIMIT: i64 = 25;
const HL_EXCHANGE_NAME: &str = "hyperliquid";

#[derive(Clone)]
pub struct AutomationWorkerConfig {
    pub poll_interval_sec: u64,
    pub worker_id: String,
    pub lease_ttl_sec: i64,
    pub hl_env: HlEnv,
}

impl AutomationWorkerConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            poll_interval_sec: config.automation_worker_poll_interval_sec,
            worker_id: config.automation_worker_id.clone(),
            lease_ttl_sec: config.automation_lease_ttl_sec,
            // TODO: thread through env (HL_ENV=mainnet|testnet) once
            // we're ready to run testnet smoke tests.
            hl_env: HlEnv::Mainnet,
        }
    }
}

/// Per-call signer scoped to a single user's HL agent wallet. Built
/// fresh inside `dispatch_intent` — short-lived, no long-running state.
struct HlAgentSigner<'a> {
    turnkey: &'a TurnkeyClient,
    suborg_id: String,
    sign_with: String,
}

#[async_trait]
impl SignerBackend for HlAgentSigner<'_> {
    async fn sign_digest(&self, digest: &[u8; 32]) -> Result<HlSignature> {
        let sig = self
            .turnkey
            .sign_raw_payload(&self.suborg_id, &self.sign_with, digest)
            .await
            .context("turnkey sign_raw_payload")?;
        Ok(HlSignature {
            r: sig.r,
            s: sig.s,
            v: sig.v_eip712(),
        })
    }
}

pub async fn run_automation_worker(
    cfg: AutomationWorkerConfig,
    config: Arc<Config>,
    db: Arc<PgPool>,
    feed: watch::Receiver<Arc<FeedSnapshot>>,
    seven_day: watch::Receiver<SevenDayApr>,
) {
    log::info!(
        "automation worker starting (worker_id={}, poll_interval={}s)",
        cfg.worker_id,
        cfg.poll_interval_sec,
    );

    let turnkey = TurnkeyClient::from_config(&config);
    let hl = HlClient::new(cfg.hl_env);

    let mut ticker = tokio::time::interval(Duration::from_secs(cfg.poll_interval_sec.max(1)));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        let intents = match AutomationService::fetch_due_intents(
            db.clone(),
            &feed,
            &seven_day,
            &cfg.worker_id,
            FETCH_LIMIT,
            cfg.lease_ttl_sec,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                log::error!("fetch_due_intents failed: {e:?}");
                continue;
            }
        };

        if intents.is_empty() {
            log::debug!("automation worker: no due intents");
            continue;
        }
        log::info!("automation worker: dispatching {} intent(s)", intents.len());

        for intent in intents {
            dispatch_intent(&intent, &hl, &turnkey, db.clone()).await;
        }
    }
}

/// Process one leased intent end-to-end. Errors here become FAILED
/// results reported back to the engine; we never bubble — bubbling
/// would skip the remaining intents in this tick.
async fn dispatch_intent(
    intent: &DueIntent,
    hl: &HlClient,
    turnkey: &TurnkeyClient,
    db: Arc<PgPool>,
) {
    let started_at_ms = chrono::Utc::now().timestamp_millis();

    let outcome = execute(intent, hl, turnkey, db.clone()).await;

    let (status, error, legs) = match outcome {
        Ok(value) => (
            "SUCCEEDED".to_string(),
            None,
            vec![IntentResultLeg {
                venue: HL_EXCHANGE_NAME.to_string(),
                ok: true,
                turnkey_activity_id: None,
                error_message: None,
                exchange_request: Some(value),
            }],
        ),
        Err(e) => {
            let msg = format!("{e:#}");
            log::error!(
                "intent {intent_id} ({action} {asset}): {msg}",
                intent_id = intent.intent_id,
                action = intent.action,
                asset = intent.asset,
            );
            (
                "FAILED".to_string(),
                Some(msg.clone()),
                vec![IntentResultLeg {
                    venue: HL_EXCHANGE_NAME.to_string(),
                    ok: false,
                    turnkey_activity_id: None,
                    error_message: Some(msg),
                    exchange_request: None,
                }],
            )
        }
    };

    let finished_at_ms = chrono::Utc::now().timestamp_millis();
    let result = IntentResult {
        status,
        node_action_id: None,
        started_at_ms: Some(started_at_ms),
        finished_at_ms: Some(finished_at_ms),
        legs,
        error_message: error,
    };

    if let Err(e) = AutomationService::record_intent_result(db, intent.intent_id, result).await {
        log::error!(
            "record_intent_result failed for {intent_id}: {e:?}",
            intent_id = intent.intent_id
        );
    }
}

/// The actual work: resolve the user's agent wallet, build the action,
/// sign and submit. Anything here that returns Err is surfaced as a
/// FAILED result via `dispatch_intent`.
async fn execute(
    intent: &DueIntent,
    hl: &HlClient,
    turnkey: &TurnkeyClient,
    db: Arc<PgPool>,
) -> Result<serde_json::Value> {
    let agent = get_hl_agent_for_user(db, intent.user_id)
        .await
        .context("lookup hl_agent_wallets")?
        .ok_or_else(|| {
            anyhow!(
                "user {} has no HL agent wallet provisioned",
                intent.user_id
            )
        })?;

    if !agent.approved_on_hl {
        return Err(anyhow!(
            "user {} has an HL agent wallet but has not yet completed approveAgent on HL",
            intent.user_id
        ));
    }

    let action = build_action(intent)?;
    let signer = HlAgentSigner {
        turnkey,
        suborg_id: agent.turnkey_suborg_id,
        sign_with: agent.evm_address,
    };
    hl.submit_action(&action, &signer).await
}

fn build_action(intent: &DueIntent) -> Result<ExchangeAction> {
    let is_long_on_hl = intent.long_exchange.eq_ignore_ascii_case(HL_EXCHANGE_NAME);
    let is_short_on_hl = intent.short_exchange.eq_ignore_ascii_case(HL_EXCHANGE_NAME);
    if !is_long_on_hl && !is_short_on_hl {
        return Err(anyhow!(
            "intent has no HL leg (long={}, short={})",
            intent.long_exchange,
            intent.short_exchange
        ));
    }

    let asset_idx = get_asset_index(&intent.asset)
        .ok_or_else(|| anyhow!("HL has no asset index for symbol {}", intent.asset))?;
    let sz_dec = get_sz_decimals(&intent.asset)
        .ok_or_else(|| anyhow!("HL has no szDecimals for symbol {}", intent.asset))?;

    let action_upper = intent.action.to_uppercase();
    let is_close = matches!(action_upper.as_str(), "CLOSE" | "EMERGENCY_CLOSE");

    // Direction. On OPEN we trade the HL leg as the engine recommended;
    // on CLOSE we flip to flatten.
    let is_buy = if is_close {
        is_short_on_hl
    } else {
        is_long_on_hl
    };

    let reference_px: f64 = intent
        .reference_price
        .px
        .parse()
        .context("parse intent reference price")?;
    let target_margin_usd: f64 = intent
        .sizing
        .target_margin_usd
        .parse()
        .context("parse target_margin_usd")?;
    let leverage = intent.sizing.leverage;

    let notional_usd = target_margin_usd * leverage;
    let size = notional_usd / reference_px;
    let size_str = format_size(size, sz_dec);

    // Aggressive IOC limit to fake a market order:
    //   buy  → ref * (1 + 1%)
    //   sell → ref * (1 - 1%)
    // TODO: honor `max_slippage_bps` from the intent config instead of 1%.
    let slip = 0.01_f64;
    let px = if is_buy {
        reference_px * (1.0 + slip)
    } else {
        reference_px * (1.0 - slip)
    };
    let px_str = format_price(px);

    let order = OrderRequest {
        a: asset_idx,
        b: is_buy,
        p: px_str,
        s: size_str,
        r: is_close,
        t: OrderTypeWire {
            limit: LimitWire {
                tif: "Ioc".to_string(),
            },
        },
    };

    Ok(ExchangeAction::Order {
        orders: vec![order],
        grouping: "na".to_string(),
    })
}

fn format_size(size: f64, sz_decimals: u32) -> String {
    let scale = 10f64.powi(sz_decimals as i32);
    let truncated = (size * scale).floor() / scale;
    format!("{:.*}", sz_decimals as usize, truncated)
}

fn format_price(px: f64) -> String {
    if px <= 0.0 {
        return "0".to_string();
    }
    let order = px.log10().floor() as i32;
    let decimals = (4 - order).clamp(0, 6) as usize;
    format!("{:.*}", decimals, px)
}
