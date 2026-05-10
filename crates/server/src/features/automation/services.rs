//! Automation feature service: thin orchestration around the pure recommender
//! and DB. Wires HTTP controllers to:
//!   - `automation_configs` upserts/reads
//!   - `automation_runs` lifecycle
//!   - `services::automation` pure best-pair logic

use std::sync::Arc;

use chrono::Utc;
use db::automation::{
    self as auto_db, action_status, action_type, run_status, AutomationConfig, AutomationRun,
    LeaseOutcome, NewAutomationAction, NewAutomationRun, UpsertAutomationConfig,
    apr_mode as cfg_apr_mode,
};
use perp_core::SevenDayApr;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    error::AppError,
    services::automation::{
        compute_best_pair, compute_config_hash, compute_emergency_close, compute_legs_hash,
        recommended_action, CurrentRunState, EffectiveConfig, Recommendation, RecommendedLeg,
        ReferencePrice, DEFAULT_LEASE_TTL_SEC, DEFAULT_POLL_INTERVAL_SEC, EVAL_INTERVAL_SEC,
        MAX_CONSECUTIVE_FAILURES_BEFORE_RUN_FAILED,
    },
    types::FeedSnapshot,
};

// ============================= Helpers =============================

pub fn parse_string_array(value: &JsonValue) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn config_to_effective(cfg: &AutomationConfig) -> EffectiveConfig {
    EffectiveConfig {
        apr_mode: cfg.apr_mode.clone(),
        min_apr_to_enter: cfg.min_apr_to_enter,
        exit_if_apr_below: cfg.exit_if_apr_below,
        rebalance_to_better_pair: cfg.rebalance_to_better_pair,
        min_rebalance_improvement_bps: cfg.min_rebalance_improvement_bps,
        min_time_between_actions_sec: cfg.min_time_between_actions_sec,
        cooldown_after_error_sec: cfg.cooldown_after_error_sec,
        max_leverage: cfg.max_leverage,
        max_actions_per_day: cfg.max_actions_per_day,
        excluded_assets: parse_string_array(&cfg.excluded_assets),
        allowed_exchanges: parse_string_array(&cfg.allowed_exchanges),
        max_slippage_bps: cfg.max_slippage_bps,
        reduce_only_on_close: cfg.reduce_only_on_close,
    }
}

pub fn run_to_effective_config(run: &AutomationRun) -> EffectiveConfig {
    // Run snapshots predate max_slippage_bps / reduce_only_on_close columns.
    // We default to the live config defaults so existing runs keep working;
    // the API contract documents that these are read from the run's user
    // config at recommendation time (not snapshotted).
    EffectiveConfig {
        apr_mode: run.apr_mode_snapshot.clone(),
        min_apr_to_enter: run.min_apr_to_enter_snapshot,
        exit_if_apr_below: run.exit_if_apr_below_snapshot,
        rebalance_to_better_pair: run.rebalance_to_better_pair_snapshot,
        min_rebalance_improvement_bps: run.min_rebalance_improvement_bps_snapshot,
        min_time_between_actions_sec: run.min_time_between_actions_sec_snapshot,
        cooldown_after_error_sec: run.cooldown_after_error_sec_snapshot,
        max_leverage: run.max_leverage_snapshot,
        max_actions_per_day: run.max_actions_per_day_snapshot,
        excluded_assets: parse_string_array(&run.excluded_assets_snapshot),
        allowed_exchanges: parse_string_array(&run.allowed_exchanges_snapshot),
        max_slippage_bps: 50,
        reduce_only_on_close: true,
    }
}

/// Run-snapshot effective config, with live-config overrides for the two
/// pass-through fields that aren't snapshotted (`max_slippage_bps`,
/// `reduce_only_on_close`). Used by the `/internal/automation/intents/due`
/// endpoint when building intent payloads.
pub fn effective_for_run_with_live_overrides(
    run: &AutomationRun,
    live: Option<&AutomationConfig>,
) -> EffectiveConfig {
    let mut eff = run_to_effective_config(run);
    if let Some(cfg) = live {
        eff.max_slippage_bps = cfg.max_slippage_bps;
        eff.reduce_only_on_close = cfg.reduce_only_on_close;
    }
    eff
}

pub fn run_to_current_state(run: &AutomationRun) -> CurrentRunState {
    let current_legs: Vec<RecommendedLeg> = run
        .current_legs
        .as_ref()
        .and_then(|v| serde_json::from_value::<Vec<RecommendedLeg>>(v.clone()).ok())
        .unwrap_or_default();
    CurrentRunState {
        current_asset: run.current_asset.clone(),
        current_legs,
        last_action_at: run
            .last_action_at
            .map(|t| chrono::DateTime::<Utc>::from_naive_utc_and_offset(t, Utc)),
        actions_today: run.actions_today,
    }
}

// ============================= Service =============================

pub struct AutomationService;

impl AutomationService {
    /// Get the user's config or default.
    pub async fn get_or_default_config(
        db: Arc<PgPool>,
        user_id: Uuid,
    ) -> Result<AutomationConfig, AppError> {
        let existing = auto_db::get_automation_config(db.clone(), user_id).await?;
        Ok(existing.unwrap_or_else(|| AutomationConfig::defaults_for(user_id)))
    }

    pub async fn upsert_config(
        db: Arc<PgPool>,
        user_id: Uuid,
        payload: UpsertAutomationConfig,
    ) -> Result<AutomationConfig, AppError> {
        let mut p = payload;
        p.user_id = user_id;
        // Normalize apr_mode to canonical values.
        p.apr_mode = match p.apr_mode.to_uppercase().as_str() {
            "SEVEN_D" | "7D" => cfg_apr_mode::SEVEN_D.to_string(),
            _ => cfg_apr_mode::NET.to_string(),
        };
        let row = auto_db::upsert_automation_config(db, &p).await?;
        Ok(row)
    }

    /// Compute a recommendation using either the run's snapshot config (if a
    /// run is provided) or the user's current `automation_configs` row.
    pub fn compute_recommendation(
        feed: &FeedSnapshot,
        seven_day: &SevenDayApr,
        cfg: &EffectiveConfig,
        current: Option<&CurrentRunState>,
        poll_interval_sec: i64,
    ) -> Recommendation {
        let now = Utc::now();
        let interval = if poll_interval_sec > 0 {
            poll_interval_sec
        } else {
            DEFAULT_POLL_INTERVAL_SEC
        };
        compute_best_pair(feed, seven_day, cfg, current, now, interval)
    }

    /// Read-only "best pair" for the user — uses their config.
    pub async fn best_pair_preview(
        db: Arc<PgPool>,
        user_id: Uuid,
        feed_rx: &watch::Receiver<Arc<FeedSnapshot>>,
        seven_day_rx: &watch::Receiver<SevenDayApr>,
        mode_override: Option<String>,
    ) -> Result<Recommendation, AppError> {
        let mut cfg = Self::get_or_default_config(db, user_id).await?;
        if let Some(mode) = mode_override {
            cfg.apr_mode = match mode.to_uppercase().as_str() {
                "SEVEN_D" | "7D" => cfg_apr_mode::SEVEN_D.to_string(),
                _ => cfg_apr_mode::NET.to_string(),
            };
        }
        let effective = config_to_effective(&cfg);

        let feed_snapshot = feed_rx.borrow().clone();
        let seven_day_snapshot = seven_day_rx.borrow().clone();

        Ok(Self::compute_recommendation(
            &feed_snapshot,
            &seven_day_snapshot,
            &effective,
            None,
            DEFAULT_POLL_INTERVAL_SEC,
        ))
    }

    /// Create + ACTIVATE a new run, snapshotting current config.
    /// Fails if the user already has an ACTIVE run.
    pub async fn create_run(
        db: Arc<PgPool>,
        user_id: Uuid,
        target_margin_usd: Option<f64>,
        leverage: Option<f64>,
    ) -> Result<Uuid, AppError> {
        if auto_db::get_active_run_for_user(db.clone(), user_id)
            .await?
            .is_some()
        {
            return Err(AppError::parse(
                "user_id",
                "user already has an ACTIVE automation run",
            ));
        }

        let cfg = Self::get_or_default_config(db.clone(), user_id).await?;
        let effective = config_to_effective(&cfg);
        let config_hash = compute_config_hash(&effective);

        let new_run = NewAutomationRun {
            id: Uuid::new_v4(),
            user_id,
            status: run_status::ACTIVE.to_string(),
            config: cfg,
            config_hash,
            target_margin_usd,
            leverage,
        };
        auto_db::create_automation_run(db, &new_run).await?;
        Ok(new_run.id)
    }

    pub async fn pause_run(db: Arc<PgPool>, run_id: Uuid) -> Result<(), AppError> {
        let updated = auto_db::update_automation_run_status_if(
            db,
            run_id,
            run_status::PAUSED,
            &[run_status::ACTIVE],
        )
        .await?;
        if !updated {
            return Err(AppError::parse(
                "status",
                "run is not ACTIVE; cannot pause",
            ));
        }
        Ok(())
    }

    pub async fn resume_run(db: Arc<PgPool>, run_id: Uuid) -> Result<(), AppError> {
        let updated = auto_db::update_automation_run_status_if(
            db,
            run_id,
            run_status::ACTIVE,
            &[run_status::PAUSED],
        )
        .await?;
        if !updated {
            return Err(AppError::parse(
                "status",
                "run is not PAUSED; cannot resume",
            ));
        }
        Ok(())
    }

    pub async fn stop_run(db: Arc<PgPool>, run_id: Uuid) -> Result<(), AppError> {
        let updated = auto_db::update_automation_run_status_if(
            db,
            run_id,
            run_status::STOPPING,
            &[run_status::ACTIVE, run_status::PAUSED],
        )
        .await?;
        if !updated {
            return Err(AppError::parse(
                "status",
                "run is not ACTIVE/PAUSED; cannot stop",
            ));
        }
        Ok(())
    }

    /// Restart a STOPPED/FAILED run by creating a new run with a fresh
    /// config snapshot. Returns the new run id.
    pub async fn restart_run(
        db: Arc<PgPool>,
        user_id: Uuid,
        previous_run_id: Uuid,
    ) -> Result<Uuid, AppError> {
        let prev = auto_db::get_automation_run(db.clone(), previous_run_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("automation run {previous_run_id}")))?;
        if prev.user_id != user_id {
            return Err(AppError::unauthorised(
                "automation run does not belong to authenticated user",
            ));
        }
        if prev.status != run_status::STOPPED && prev.status != run_status::FAILED {
            return Err(AppError::parse(
                "status",
                format!(
                    "run must be STOPPED or FAILED to restart, got {}",
                    prev.status
                ),
            ));
        }
        Self::create_run(db, user_id, prev.target_margin_usd, prev.leverage).await
    }

    pub async fn list_runs(
        db: Arc<PgPool>,
        user_id: Uuid,
    ) -> Result<Vec<AutomationRun>, AppError> {
        Ok(auto_db::get_automation_runs_by_user(db, user_id).await?)
    }

    pub async fn get_run(
        db: Arc<PgPool>,
        user_id: Uuid,
        run_id: Uuid,
    ) -> Result<AutomationRun, AppError> {
        let run = auto_db::get_automation_run(db, run_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("automation run {run_id}")))?;
        if run.user_id != user_id {
            return Err(AppError::unauthorised(
                "automation run does not belong to authenticated user",
            ));
        }
        Ok(run)
    }

    /// Fetch up to `limit` actionable intents for the Node executor.
    ///
    /// For each due ACTIVE run we run the recommendation engine and emit an
    /// intent on OPEN/CLOSE/REBALANCE. STOPPING runs that still hold a
    /// position get an EMERGENCY_CLOSE intent (no cooldown applies).
    /// Inserts into `automation_actions` are idempotent on the standard
    /// composite key; if a row already exists we attempt to lease it
    /// (skipping rows already terminal or actively leased by someone else).
    pub async fn fetch_due_intents(
        db: Arc<PgPool>,
        feed_rx: &watch::Receiver<Arc<FeedSnapshot>>,
        seven_day_rx: &watch::Receiver<SevenDayApr>,
        worker_id: &str,
        limit: i64,
        lease_ttl_sec: i64,
    ) -> Result<Vec<DueIntent>, AppError> {
        let feed = feed_rx.borrow().clone();
        let seven = seven_day_rx.borrow().clone();
        let mut tx = db.begin().await.map_err(anyhow::Error::from)?;

        let mut runs =
            auto_db::fetch_due_active_runs_tx(&mut tx, EVAL_INTERVAL_SEC, limit).await?;
        let stopping = auto_db::fetch_stopping_runs_with_position_tx(&mut tx, limit).await?;
        runs.extend(stopping);

        let now = Utc::now();
        let mut intents: Vec<DueIntent> = Vec::new();

        for run in runs.iter() {
            // Build the recommendation: STOPPING+position → EMERGENCY_CLOSE
            // override; otherwise the normal pipeline.
            let rec = if run.status == run_status::STOPPING && run.current_asset.is_some() {
                let legs: Vec<RecommendedLeg> = run
                    .current_legs
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let asset = run.current_asset.clone().unwrap_or_default();
                compute_emergency_close(
                    &feed,
                    &run.apr_mode_snapshot,
                    &asset,
                    &legs,
                    now,
                    DEFAULT_POLL_INTERVAL_SEC,
                )
            } else {
                let live_cfg = auto_db::get_automation_config(db.clone(), run.user_id).await?;
                let effective = effective_for_run_with_live_overrides(run, live_cfg.as_ref());
                let current = run_to_current_state(run);
                compute_best_pair(
                    &feed,
                    &seven,
                    &effective,
                    Some(&current),
                    now,
                    DEFAULT_POLL_INTERVAL_SEC,
                )
            };

            // Always touch last_recommendation_at so we don't busy-loop on the run.
            auto_db::touch_run_recommendation_tx(
                &mut tx,
                run.id,
                rec.decision_id,
                &rec.decision_hash,
            )
            .await?;

            // Skip non-actionable.
            let action_type_str = match rec.recommended_action.as_str() {
                recommended_action::OPEN => action_type::OPEN,
                recommended_action::CLOSE => action_type::CLOSE,
                recommended_action::REBALANCE => action_type::REBALANCE,
                recommended_action::EMERGENCY_CLOSE => action_type::EMERGENCY_CLOSE,
                _ => continue,
            };
            let asset = match rec.asset.as_deref() {
                Some(a) => a.to_string(),
                None => continue,
            };
            // Node uses referencePrice for sizing + IOC/limit logic. If we don't have a
            // usable mark price, do not emit an executable intent.
            let Some(reference_price) = rec.reference_price.clone() else {
                continue;
            };
            let legs_value = serde_json::to_value(&rec.legs).map_err(AppError::from)?;
            let legs_hash = compute_legs_hash(&rec.legs);

            let new_action = NewAutomationAction {
                id: Uuid::new_v4(),
                run_id: run.id,
                decision_id: rec.decision_id,
                decision_hash: rec.decision_hash.clone(),
                as_of_bucket: rec.as_of_bucket,
                action_type: action_type_str.to_string(),
                asset: asset.clone(),
                legs: legs_value.clone(),
                legs_hash: legs_hash.clone(),
                hedge_intent_id: None,
                payload: None,
            };

            // Insert-or-find; then lease.
            let leased = match auto_db::insert_action_if_new_tx(&mut tx, &new_action).await? {
                Some(id) => {
                    let row = auto_db::lease_action_tx(&mut tx, id, worker_id, lease_ttl_sec)
                        .await?;
                    Some(row)
                }
                None => {
                    let existing = auto_db::find_action_by_idempotency_tx(
                        &mut tx,
                        run.id,
                        action_type_str,
                        &asset,
                        &legs_hash,
                        rec.as_of_bucket,
                    )
                    .await?;
                    match existing.as_ref().map(|a| {
                        auto_db::classify_existing_action(a, now.naive_utc())
                    }) {
                        Some(LeaseOutcome::Leased(a)) => {
                            let row = auto_db::lease_action_tx(
                                &mut tx,
                                a.id,
                                worker_id,
                                lease_ttl_sec,
                            )
                            .await?;
                            Some(row)
                        }
                        _ => None,
                    }
                }
            };

            let Some(action_row) = leased else {
                continue;
            };

            // Determine effective config (pass-through fields from live cfg).
            let live_cfg = auto_db::get_automation_config(db.clone(), run.user_id).await?;
            let max_slippage_bps = live_cfg
                .as_ref()
                .map(|c| c.max_slippage_bps)
                .unwrap_or(50);
            let reduce_only_on_close = live_cfg
                .as_ref()
                .map(|c| c.reduce_only_on_close)
                .unwrap_or(true);

            let long_exchange = rec
                .legs
                .iter()
                .find(|l| l.side == "LONG")
                .map(|l| l.exchange.clone())
                .unwrap_or_default();
            let short_exchange = rec
                .legs
                .iter()
                .find(|l| l.side == "SHORT")
                .map(|l| l.exchange.clone())
                .unwrap_or_default();

            intents.push(DueIntent {
                intent_id: action_row.id,
                user_id: run.user_id,
                run_id: run.id,
                as_of_ms: rec.as_of_ts * 1000,
                as_of_bucket: rec.as_of_bucket,
                action: action_type_str.to_string(),
                apr_mode: rec.metric_mode.clone(),
                metric_value_apr_pct: rec.metric_value_apr_pct,
                asset: asset.clone(),
                long_exchange,
                short_exchange,
                reference_price,
                sizing: IntentSizing {
                    target_margin_usd: format_money(
                        run.target_margin_usd
                            .unwrap_or(run.max_position_size_usd_snapshot),
                    ),
                    leverage: run.leverage.unwrap_or(run.max_leverage_snapshot),
                    max_position_size_usd: format_money(run.max_position_size_usd_snapshot),
                },
                constraints: IntentConstraints {
                    max_slippage_bps,
                    reduce_only_on_close,
                },
            });
        }

        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(intents)
    }

    /// Apply a Node-reported execution result. Idempotent: if the action is
    /// already in a terminal state, returns 200 with `accepted=false`.
    pub async fn record_intent_result(
        db: Arc<PgPool>,
        intent_id: Uuid,
        body: IntentResult,
    ) -> Result<IntentResultAck, AppError> {
        // Validate status
        let status_norm = match body.status.to_uppercase().as_str() {
            action_status::SUCCEEDED | "SUCCESS" => action_status::SUCCEEDED,
            action_status::FAILED => action_status::FAILED,
            action_status::PARTIAL_FAILURE => action_status::PARTIAL_FAILURE,
            other => {
                return Err(AppError::parse(
                    "status",
                    format!(
                        "must be SUCCEEDED|FAILED|PARTIAL_FAILURE, got {other}"
                    ),
                ));
            }
        };

        let started_at = body
            .started_at_ms
            .and_then(chrono::DateTime::<Utc>::from_timestamp_millis)
            .map(|d| d.naive_utc());
        let finished_at = body
            .finished_at_ms
            .and_then(chrono::DateTime::<Utc>::from_timestamp_millis)
            .map(|d| d.naive_utc());

        let mut tx = db.begin().await.map_err(anyhow::Error::from)?;

        let result_value = serde_json::to_value(&body).map_err(AppError::from)?;
        let updated = auto_db::complete_action_with_result_tx(
            &mut tx,
            intent_id,
            status_norm,
            body.node_action_id,
            started_at,
            finished_at,
            body.error_message.as_deref(),
            &result_value,
        )
        .await?;

        let Some(action) = updated else {
            // Either the action doesn't exist, or it was already terminal.
            tx.commit().await.map_err(anyhow::Error::from)?;
            let exists = auto_db::get_automation_action(db.clone(), intent_id).await?;
            return Ok(IntentResultAck {
                intent_id,
                accepted: false,
                run_status: exists.map(|_| "no_change".to_string()),
            });
        };

        // Look up run for state apply.
        let run = auto_db::fetch_run_for_update_tx(&mut tx, action.run_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("automation run {}", action.run_id)))?;

        let new_run_status = match (status_norm, action.action_type.as_str()) {
            (action_status::SUCCEEDED, action_type::OPEN | action_type::REBALANCE) => {
                let new_legs = action.legs.clone();
                let mut transition_to: Option<&str> = None;
                if run.status == run_status::STOPPING {
                    // Should not happen for OPEN/REBALANCE while STOPPING, but
                    // be defensive: leave run in STOPPING; the EMERGENCY_CLOSE
                    // pass will pick it up next.
                    transition_to = None;
                }
                auto_db::apply_run_success_tx(
                    &mut tx,
                    run.id,
                    Some(&action.asset),
                    Some(&new_legs),
                    false,
                    transition_to,
                )
                .await?;
                run.status.clone()
            }
            (
                action_status::SUCCEEDED,
                action_type::CLOSE | action_type::EMERGENCY_CLOSE,
            ) => {
                let transition_to = if run.status == run_status::STOPPING {
                    Some(run_status::STOPPED)
                } else {
                    None
                };
                auto_db::apply_run_success_tx(
                    &mut tx,
                    run.id,
                    None,
                    None,
                    true,
                    transition_to,
                )
                .await?;
                transition_to
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| run.status.clone())
            }
            (action_status::FAILED | action_status::PARTIAL_FAILURE, _) => {
                let err_msg = body
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "execution failed".to_string());
                let consec = auto_db::apply_run_failure_tx(
                    &mut tx,
                    run.id,
                    &err_msg,
                    MAX_CONSECUTIVE_FAILURES_BEFORE_RUN_FAILED,
                )
                .await?;
                if consec >= MAX_CONSECUTIVE_FAILURES_BEFORE_RUN_FAILED {
                    run_status::FAILED.to_string()
                } else {
                    run.status.clone()
                }
            }
            _ => run.status.clone(),
        };

        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(IntentResultAck {
            intent_id,
            accepted: true,
            run_status: Some(new_run_status),
        })
    }
}

/// JSON-friendly money formatter — string decimal, full precision.
fn format_money(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.10}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// ============================= Internal API DTOs =============================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueIntent {
    pub intent_id: Uuid,
    pub user_id: Uuid,
    pub run_id: Uuid,
    pub as_of_ms: i64,
    pub as_of_bucket: i64,
    pub action: String,
    pub apr_mode: String,
    pub metric_value_apr_pct: f64,
    pub asset: String,
    pub long_exchange: String,
    pub short_exchange: String,
    pub reference_price: ReferencePrice,
    pub sizing: IntentSizing,
    pub constraints: IntentConstraints,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentSizing {
    pub target_margin_usd: String,
    pub leverage: f64,
    pub max_position_size_usd: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentConstraints {
    pub max_slippage_bps: i32,
    pub reduce_only_on_close: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct IntentResult {
    #[validate(length(min = 1, message = "status is required"))]
    pub status: String,
    #[serde(default)]
    pub node_action_id: Option<Uuid>,
    #[serde(default)]
    pub started_at_ms: Option<i64>,
    #[serde(default)]
    pub finished_at_ms: Option<i64>,
    #[serde(default)]
    pub legs: Vec<IntentResultLeg>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentResultLeg {
    pub venue: String,
    pub ok: bool,
    #[serde(default)]
    pub turnkey_activity_id: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub exchange_request: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentResultAck {
    pub intent_id: Uuid,
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_status: Option<String>,
}

/// Default lease TTL exposed for the controller to pick up from config.
pub const FALLBACK_LEASE_TTL_SEC: i64 = DEFAULT_LEASE_TTL_SEC;

