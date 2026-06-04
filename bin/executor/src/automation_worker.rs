//! Automation worker — in-process Rust executor for the planned Node
//! worker referenced in V13 (never built).
//!
//! Flow per tick:
//!   1. `AutomationService::fetch_due_intents` returns intents whose APR
//!      crossed the user's threshold AND which aren't already in position
//!      (the engine checks `current_legs`). Each intent is also leased
//!      to this worker.
//!   2. For each intent we pick which leg to execute. v1 picks ONE leg —
//!      preferring Hyperliquid, falling back to Pacifica. Multi-leg
//!      simultaneous execution (true delta-neutral) is a future task.
//!   3. Look up the user's agent wallet for that venue (`hl_agent_wallets`
//!      or `pacifica_agent_wallets`). Both venues use per-user agent
//!      wallets so a compromised key never affects more than one user.
//!   4. Build the venue-specific order, sign via Turnkey, submit, report
//!      back via `record_intent_result`. SUCCEEDED on OPEN causes the
//!      engine to write `current_legs` — the "marks opened" idempotency
//!      marker that prevents re-opening.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use db::wallet::queries::{get_hl_agent_for_user, get_pacifica_agent_for_user};
use hyperliquid::{
    helpers::markets::{get_asset_index, get_sz_decimals},
    services::exchange::{
        ExchangeAction, HlClient, HlEnv, HlSignature, LimitWire, OrderRequest, OrderTypeWire,
        SignerBackend as HlSigner,
    },
};
use pacifica::services::exchange::{
    MarketOrderParams, PacificaClient, Side as PacificaSide, SignerBackend as PacificaSigner,
};
use perp_core::{SevenDayApr, config::Config};
use server::{
    features::automation::services::{AutomationService, DueIntent, IntentResult, IntentResultLeg},
    services::turnkey::TurnkeyClient,
    types::FeedSnapshot,
};
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

const FETCH_LIMIT: i64 = 25;
const HL_EXCHANGE_NAME: &str = "hyperliquid";
const PACIFICA_EXCHANGE_NAME: &str = "pacifica";

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

/// HL signer adapter: turns Turnkey's `(r, s, v)` into the EIP-712 form
/// HL expects (v bumped to 27/28).
struct HlAgentSigner<'a> {
    turnkey: &'a TurnkeyClient,
    suborg_id: String,
    sign_with: String,
}

#[async_trait]
impl HlSigner for HlAgentSigner<'_> {
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

/// Pacifica signer adapter: signs raw UTF-8 bytes with the user's Ed25519
/// Turnkey wallet, returns the 64-byte signature directly (Pacifica module
/// handles base58 encoding).
struct PacificaAgentSigner<'a> {
    turnkey: &'a TurnkeyClient,
    suborg_id: String,
    sign_with: String,
}

#[async_trait]
impl PacificaSigner for PacificaAgentSigner<'_> {
    async fn sign_message(&self, message: &[u8]) -> Result<[u8; 64]> {
        self.turnkey
            .sign_solana_payload(&self.suborg_id, &self.sign_with, message)
            .await
            .context("turnkey sign_solana_payload")
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
    let pacifica = PacificaClient::new();

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
            dispatch_intent(&intent, &hl, &pacifica, &turnkey, db.clone()).await;
        }
    }
}

/// Which venue, on which side, this intent's executable leg is on.
struct ChosenLeg {
    venue: Venue,
    is_buy_on_open: bool,
}

enum Venue {
    Hyperliquid,
    Pacifica,
}

impl Venue {
    fn name(&self) -> &'static str {
        match self {
            Venue::Hyperliquid => HL_EXCHANGE_NAME,
            Venue::Pacifica => PACIFICA_EXCHANGE_NAME,
        }
    }
}

/// Pick which venue's leg to execute. v1 prefers HL, falls back to
/// Pacifica. Returns `None` if neither venue is named in the intent.
fn pick_leg(intent: &DueIntent) -> Option<ChosenLeg> {
    for (venue, name) in [
        (Venue::Hyperliquid, HL_EXCHANGE_NAME),
        (Venue::Pacifica, PACIFICA_EXCHANGE_NAME),
    ] {
        let is_long = intent.long_exchange.eq_ignore_ascii_case(name);
        let is_short = intent.short_exchange.eq_ignore_ascii_case(name);
        if is_long || is_short {
            return Some(ChosenLeg {
                venue,
                is_buy_on_open: is_long,
            });
        }
    }
    None
}

/// Process one leased intent end-to-end. We never bubble errors — every
/// failure becomes a FAILED `record_intent_result` so the engine sees a
/// terminal state.
async fn dispatch_intent(
    intent: &DueIntent,
    hl: &HlClient,
    pacifica: &PacificaClient,
    turnkey: &TurnkeyClient,
    db: Arc<PgPool>,
) {
    let started_at_ms = chrono::Utc::now().timestamp_millis();

    let leg = match pick_leg(intent) {
        Some(l) => l,
        None => {
            report_failure(
                intent,
                "intent has no HL or Pacifica leg",
                "unknown",
                started_at_ms,
                db,
            )
            .await;
            return;
        }
    };

    let outcome = match leg.venue {
        Venue::Hyperliquid => {
            execute_hl(intent, &leg, hl, turnkey, db.clone()).await
        }
        Venue::Pacifica => {
            execute_pacifica(intent, &leg, pacifica, turnkey, db.clone()).await
        }
    };

    let venue_name = leg.venue.name();
    let (status, error, legs) = match outcome {
        Ok(value) => (
            "SUCCEEDED".to_string(),
            None,
            vec![IntentResultLeg {
                venue: venue_name.to_string(),
                ok: true,
                turnkey_activity_id: None,
                error_message: None,
                exchange_request: Some(value),
            }],
        ),
        Err(e) => {
            let msg = format!("{e:#}");
            log::error!(
                "intent {intent_id} ({action} {asset} on {venue_name}): {msg}",
                intent_id = intent.intent_id,
                action = intent.action,
                asset = intent.asset,
            );
            (
                "FAILED".to_string(),
                Some(msg.clone()),
                vec![IntentResultLeg {
                    venue: venue_name.to_string(),
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

/// Report a FAILED result with no exchange request — used when we couldn't
/// even build the order (e.g. unknown venue, missing agent wallet pre-check).
async fn report_failure(
    intent: &DueIntent,
    msg: &str,
    venue: &str,
    started_at_ms: i64,
    db: Arc<PgPool>,
) {
    log::error!(
        "intent {intent_id} ({action} {asset}): {msg}",
        intent_id = intent.intent_id,
        action = intent.action,
        asset = intent.asset,
    );
    let result = IntentResult {
        status: "FAILED".to_string(),
        node_action_id: None,
        started_at_ms: Some(started_at_ms),
        finished_at_ms: Some(chrono::Utc::now().timestamp_millis()),
        legs: vec![IntentResultLeg {
            venue: venue.to_string(),
            ok: false,
            turnkey_activity_id: None,
            error_message: Some(msg.to_string()),
            exchange_request: None,
        }],
        error_message: Some(msg.to_string()),
    };
    if let Err(e) = AutomationService::record_intent_result(db, intent.intent_id, result).await {
        log::error!("record_intent_result failed: {e:?}");
    }
}

// ────────── Hyperliquid path ──────────

async fn execute_hl(
    intent: &DueIntent,
    leg: &ChosenLeg,
    hl: &HlClient,
    turnkey: &TurnkeyClient,
    db: Arc<PgPool>,
) -> Result<serde_json::Value> {
    let agent = get_hl_agent_for_user(db, intent.user_id)
        .await
        .context("lookup hl_agent_wallets")?
        .ok_or_else(|| anyhow!("user {} has no HL agent wallet provisioned", intent.user_id))?;

    if !agent.approved_on_hl {
        return Err(anyhow!(
            "user {} has an HL agent wallet but has not completed approveAgent on HL",
            intent.user_id
        ));
    }

    let action = build_hl_action(intent, leg)?;
    let signer = HlAgentSigner {
        turnkey,
        suborg_id: agent.turnkey_suborg_id,
        sign_with: agent.evm_address,
    };
    hl.submit_action(&action, &signer).await
}

fn build_hl_action(intent: &DueIntent, leg: &ChosenLeg) -> Result<ExchangeAction> {
    let asset_idx = get_asset_index(&intent.asset)
        .ok_or_else(|| anyhow!("HL has no asset index for symbol {}", intent.asset))?;
    let sz_dec = get_sz_decimals(&intent.asset)
        .ok_or_else(|| anyhow!("HL has no szDecimals for symbol {}", intent.asset))?;

    let is_close = matches!(intent.action.to_uppercase().as_str(), "CLOSE" | "EMERGENCY_CLOSE");
    let is_buy = if is_close { !leg.is_buy_on_open } else { leg.is_buy_on_open };

    let (size_str, px_str) = compute_size_and_price(intent, sz_dec, is_buy)?;

    Ok(ExchangeAction::Order {
        orders: vec![OrderRequest {
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
        }],
        grouping: "na".to_string(),
    })
}

// ────────── Pacifica path ──────────

async fn execute_pacifica(
    intent: &DueIntent,
    leg: &ChosenLeg,
    pacifica: &PacificaClient,
    turnkey: &TurnkeyClient,
    db: Arc<PgPool>,
) -> Result<serde_json::Value> {
    let agent = get_pacifica_agent_for_user(db.clone(), intent.user_id)
        .await
        .context("lookup pacifica_agent_wallets")?
        .ok_or_else(|| {
            anyhow!(
                "user {} has no Pacifica agent wallet provisioned",
                intent.user_id
            )
        })?;

    if !agent.approved_on_pacifica {
        return Err(anyhow!(
            "user {} has a Pacifica agent wallet but the agent has not been authorized on Pacifica yet",
            intent.user_id
        ));
    }

    let user_solana_pubkey = lookup_user_solana_pubkey(db, intent.user_id).await?;

    let is_close = matches!(intent.action.to_uppercase().as_str(), "CLOSE" | "EMERGENCY_CLOSE");
    let is_buy = if is_close { !leg.is_buy_on_open } else { leg.is_buy_on_open };

    // Pacifica's `szDecimals` analog isn't surfaced from a static table
    // here; defer to a fixed precision for the v1 string format. If
    // Pacifica rejects with "invalid amount precision", we'll tighten
    // this against their per-symbol metadata.
    let (size_str, _px_str) = compute_size_and_price(intent, 6, is_buy)?;

    let params = MarketOrderParams {
        symbol: intent.asset.clone(),
        side: if is_buy { PacificaSide::Bid } else { PacificaSide::Ask },
        amount: size_str,
        // Hard-coded for v1 — see TODO in `compute_size_and_price`.
        slippage_percent: "1.0".to_string(),
        reduce_only: is_close,
        client_order_id: Some(Uuid::new_v4().to_string()),
    };

    let signer = PacificaAgentSigner {
        turnkey,
        suborg_id: agent.turnkey_suborg_id,
        sign_with: agent.solana_pubkey.clone(),
    };

    pacifica
        .create_market_order(&user_solana_pubkey, &agent.solana_pubkey, &params, &signer)
        .await
}

/// Look up the user's main Solana wallet address (the master account
/// Pacifica needs in the `account` field). Pulled from the `wallets`
/// table via the existing `users.wallet_id` join.
async fn lookup_user_solana_pubkey(db: Arc<PgPool>, user_id: Uuid) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT w.turnkey_solana_address
        FROM users u
        JOIN wallets w ON w.id = u.wallet_id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&*db)
    .await
    .context("lookup user solana address")?;

    let (addr,) = row.ok_or_else(|| anyhow!("user {user_id} has no wallet row"))?;
    if addr.is_empty() {
        return Err(anyhow!("user {user_id} has no Solana address"));
    }
    Ok(addr)
}

// ────────── Shared helpers ──────────

/// Compute `(size_string, aggressive_limit_price_string)` from the
/// intent's sizing + reference price. Used by both venues; HL consumes
/// the size, Pacifica throws away the price (it submits a "market"
/// order with a slippage cap instead).
fn compute_size_and_price(
    intent: &DueIntent,
    sz_decimals: u32,
    is_buy: bool,
) -> Result<(String, String)> {
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
    let size_str = format_size(size, sz_decimals);

    // TODO: honor `max_slippage_bps` from the intent config.
    let slip = 0.01_f64;
    let px = if is_buy {
        reference_px * (1.0 + slip)
    } else {
        reference_px * (1.0 - slip)
    };
    Ok((size_str, format_price(px)))
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
