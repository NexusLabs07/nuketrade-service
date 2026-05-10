use std::sync::Arc;

use chrono::NaiveDateTime;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::automation::models::{
    action_status, AutomationAction, AutomationConfig, AutomationRun, LeaseOutcome,
    NewAutomationAction, NewAutomationRun, UpsertAutomationConfig,
};

// ============================= Configs =============================

pub async fn get_automation_config(
    db: Arc<PgPool>,
    user_id: Uuid,
) -> Result<Option<AutomationConfig>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationConfig>(
        "SELECT * FROM automation_configs WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&*db)
    .await?;
    Ok(row)
}

pub async fn upsert_automation_config(
    db: Arc<PgPool>,
    payload: &UpsertAutomationConfig,
) -> Result<AutomationConfig, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationConfig>(
        r#"
        INSERT INTO automation_configs (
            user_id, apr_mode, min_apr_to_enter, exit_if_apr_below, rebalance_to_better_pair,
            min_rebalance_improvement_bps, min_time_between_actions_sec, cooldown_after_error_sec,
            max_position_size_usd, max_leverage, max_actions_per_day,
            excluded_assets, allowed_exchanges,
            max_slippage_bps, reduce_only_on_close
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (user_id) DO UPDATE SET
            apr_mode = EXCLUDED.apr_mode,
            min_apr_to_enter = EXCLUDED.min_apr_to_enter,
            exit_if_apr_below = EXCLUDED.exit_if_apr_below,
            rebalance_to_better_pair = EXCLUDED.rebalance_to_better_pair,
            min_rebalance_improvement_bps = EXCLUDED.min_rebalance_improvement_bps,
            min_time_between_actions_sec = EXCLUDED.min_time_between_actions_sec,
            cooldown_after_error_sec = EXCLUDED.cooldown_after_error_sec,
            max_position_size_usd = EXCLUDED.max_position_size_usd,
            max_leverage = EXCLUDED.max_leverage,
            max_actions_per_day = EXCLUDED.max_actions_per_day,
            excluded_assets = EXCLUDED.excluded_assets,
            allowed_exchanges = EXCLUDED.allowed_exchanges,
            max_slippage_bps = EXCLUDED.max_slippage_bps,
            reduce_only_on_close = EXCLUDED.reduce_only_on_close,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(payload.user_id)
    .bind(&payload.apr_mode)
    .bind(payload.min_apr_to_enter)
    .bind(payload.exit_if_apr_below)
    .bind(payload.rebalance_to_better_pair)
    .bind(payload.min_rebalance_improvement_bps)
    .bind(payload.min_time_between_actions_sec)
    .bind(payload.cooldown_after_error_sec)
    .bind(payload.max_position_size_usd)
    .bind(payload.max_leverage)
    .bind(payload.max_actions_per_day)
    .bind(&payload.excluded_assets)
    .bind(&payload.allowed_exchanges)
    .bind(payload.max_slippage_bps)
    .bind(payload.reduce_only_on_close)
    .fetch_one(&*db)
    .await?;

    Ok(row)
}

// ============================= Runs =============================

pub async fn create_automation_run(
    db: Arc<PgPool>,
    new_run: &NewAutomationRun,
) -> Result<Uuid, anyhow::Error> {
    let cfg = &new_run.config;
    sqlx::query(
        r#"
        INSERT INTO automation_runs (
            id, user_id, status,
            apr_mode_snapshot, min_apr_to_enter_snapshot, exit_if_apr_below_snapshot,
            rebalance_to_better_pair_snapshot, min_rebalance_improvement_bps_snapshot,
            min_time_between_actions_sec_snapshot, cooldown_after_error_sec_snapshot,
            max_position_size_usd_snapshot, max_leverage_snapshot, max_actions_per_day_snapshot,
            excluded_assets_snapshot, allowed_exchanges_snapshot, config_hash,
            target_margin_usd, leverage
        )
        VALUES (
            $1, $2, $3,
            $4, $5, $6,
            $7, $8,
            $9, $10,
            $11, $12, $13,
            $14, $15, $16,
            $17, $18
        )
        "#,
    )
    .bind(new_run.id)
    .bind(new_run.user_id)
    .bind(&new_run.status)
    .bind(&cfg.apr_mode)
    .bind(cfg.min_apr_to_enter)
    .bind(cfg.exit_if_apr_below)
    .bind(cfg.rebalance_to_better_pair)
    .bind(cfg.min_rebalance_improvement_bps)
    .bind(cfg.min_time_between_actions_sec)
    .bind(cfg.cooldown_after_error_sec)
    .bind(cfg.max_position_size_usd)
    .bind(cfg.max_leverage)
    .bind(cfg.max_actions_per_day)
    .bind(&cfg.excluded_assets)
    .bind(&cfg.allowed_exchanges)
    .bind(&new_run.config_hash)
    .bind(new_run.target_margin_usd)
    .bind(new_run.leverage)
    .execute(&*db)
    .await?;

    Ok(new_run.id)
}

pub async fn get_automation_run(
    db: Arc<PgPool>,
    id: Uuid,
) -> Result<Option<AutomationRun>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationRun>("SELECT * FROM automation_runs WHERE id = $1")
        .bind(id)
        .fetch_optional(&*db)
        .await?;
    Ok(row)
}

pub async fn get_automation_runs_by_user(
    db: Arc<PgPool>,
    user_id: Uuid,
) -> Result<Vec<AutomationRun>, anyhow::Error> {
    let rows = sqlx::query_as::<_, AutomationRun>(
        "SELECT * FROM automation_runs WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&*db)
    .await?;
    Ok(rows)
}

pub async fn get_active_run_for_user(
    db: Arc<PgPool>,
    user_id: Uuid,
) -> Result<Option<AutomationRun>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationRun>(
        "SELECT * FROM automation_runs WHERE user_id = $1 AND status = 'ACTIVE' LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&*db)
    .await?;
    Ok(row)
}

pub async fn update_automation_run_status(
    db: Arc<PgPool>,
    id: Uuid,
    status: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query("UPDATE automation_runs SET status = $1, updated_at = now() WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(&*db)
        .await?;
    Ok(())
}

/// Conditional status transition. Returns true if the row was updated.
pub async fn update_automation_run_status_if(
    db: Arc<PgPool>,
    id: Uuid,
    new_status: &str,
    allowed_from: &[&str],
) -> Result<bool, anyhow::Error> {
    let result = sqlx::query(
        "UPDATE automation_runs SET status = $1, updated_at = now() \
         WHERE id = $2 AND status = ANY($3)",
    )
    .bind(new_status)
    .bind(id)
    .bind(allowed_from)
    .execute(&*db)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn record_run_error(
    db: Arc<PgPool>,
    id: Uuid,
    error: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE automation_runs
        SET last_error = $1,
            last_error_at = now(),
            updated_at = now()
        WHERE id = $2
        "#,
    )
    .bind(error)
    .bind(id)
    .execute(&*db)
    .await?;
    Ok(())
}

// ============================= Due-runs polling =============================
//
// The /internal/automation/intents/due endpoint runs the recommendation
// engine on the runs returned here. We must only include rows that are not
// in cooldown and not too recently evaluated, and we use FOR UPDATE SKIP
// LOCKED so multiple Node workers polling the API can run in parallel
// without seeing the same run twice.

/// Pick due ACTIVE runs for evaluation. Caller must run this inside a
/// transaction (locks are held until the tx commits/aborts).
pub async fn fetch_due_active_runs_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    eval_interval_sec: i64,
    limit: i64,
) -> Result<Vec<AutomationRun>, anyhow::Error> {
    let rows = sqlx::query_as::<_, AutomationRun>(
        r#"
        SELECT *
        FROM automation_runs
        WHERE status = 'ACTIVE'
          AND (last_recommendation_at IS NULL
               OR last_recommendation_at < now() - make_interval(secs => $1))
          AND (last_error_at IS NULL
               OR last_error_at < now() - make_interval(secs => cooldown_after_error_sec_snapshot))
        ORDER BY last_recommendation_at NULLS FIRST
        LIMIT $2
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(eval_interval_sec)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

/// Pick STOPPING runs that still have a position (need an emergency close).
/// Bypasses normal cooldown.
pub async fn fetch_stopping_runs_with_position_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    limit: i64,
) -> Result<Vec<AutomationRun>, anyhow::Error> {
    let rows = sqlx::query_as::<_, AutomationRun>(
        r#"
        SELECT *
        FROM automation_runs
        WHERE status = 'STOPPING'
          AND current_asset IS NOT NULL
        ORDER BY updated_at ASC
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

/// Update last_recommendation_at + last_decision_* for a run we evaluated.
pub async fn touch_run_recommendation_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    decision_id: Uuid,
    decision_hash: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE automation_runs
        SET last_recommendation_at = now(),
            last_decision_id = $1,
            last_decision_hash = $2,
            updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(decision_id)
    .bind(decision_hash)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

// ============================= Actions: lease / result =============================

/// Try to insert a fresh action row. If the idempotency key already exists,
/// returns `Ok(None)` and the caller should fall back to inspecting the
/// existing row via [`fetch_action_for_update_tx`].
pub async fn insert_action_if_new_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new_action: &NewAutomationAction,
) -> Result<Option<Uuid>, anyhow::Error> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO automation_actions (
            id, run_id, decision_id, decision_hash, as_of_bucket,
            action_type, asset, legs, legs_hash, hedge_intent_id, payload, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'PENDING')
        ON CONFLICT (run_id, action_type, asset, legs_hash, as_of_bucket) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(new_action.id)
    .bind(new_action.run_id)
    .bind(new_action.decision_id)
    .bind(&new_action.decision_hash)
    .bind(new_action.as_of_bucket)
    .bind(&new_action.action_type)
    .bind(&new_action.asset)
    .bind(&new_action.legs)
    .bind(&new_action.legs_hash)
    .bind(new_action.hedge_intent_id)
    .bind(&new_action.payload)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Look up an action by idempotency key (used after `INSERT … ON CONFLICT
/// DO NOTHING` returned no rows).
pub async fn find_action_by_idempotency_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: Uuid,
    action_type: &str,
    asset: &str,
    legs_hash: &str,
    as_of_bucket: i64,
) -> Result<Option<AutomationAction>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationAction>(
        r#"
        SELECT *
        FROM automation_actions
        WHERE run_id = $1 AND action_type = $2 AND asset = $3
          AND legs_hash = $4 AND as_of_bucket = $5
        FOR UPDATE
        "#,
    )
    .bind(run_id)
    .bind(action_type)
    .bind(asset)
    .bind(legs_hash)
    .bind(as_of_bucket)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

pub async fn fetch_action_for_update_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> Result<Option<AutomationAction>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationAction>(
        "SELECT * FROM automation_actions WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

/// Take the lease on `id` for `worker` (`leased_until = now() + ttl`). Returns
/// the updated row.
pub async fn lease_action_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    worker: &str,
    lease_ttl_sec: i64,
) -> Result<AutomationAction, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationAction>(
        r#"
        UPDATE automation_actions
        SET status = 'IN_PROGRESS',
            leased_by = $1,
            leased_until = now() + make_interval(secs => $2),
            started_at = COALESCE(started_at, now()),
            updated_at = now()
        WHERE id = $3
        RETURNING *
        "#,
    )
    .bind(worker)
    .bind(lease_ttl_sec)
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}

/// Consider an existing action row and decide whether it can be re-leased.
/// Used by `/due` after `INSERT … ON CONFLICT DO NOTHING` returned no rows.
pub fn classify_existing_action(action: &AutomationAction, now: NaiveDateTime) -> LeaseOutcome {
    match action.status.as_str() {
        action_status::SUCCEEDED | action_status::FAILED | action_status::PARTIAL_FAILURE => {
            LeaseOutcome::Terminal
        }
        action_status::IN_PROGRESS => match action.leased_until {
            Some(until) if until > now => LeaseOutcome::BusyElsewhere,
            _ => LeaseOutcome::Leased(action.clone()),
        },
        // PENDING or anything else → re-lease.
        _ => LeaseOutcome::Leased(action.clone()),
    }
}

/// Persist a Node-reported execution result. Idempotent: if the action is
/// already terminal, returns `Ok(None)`.
#[allow(clippy::too_many_arguments)]
pub async fn complete_action_with_result_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    new_status: &str,
    node_action_id: Option<Uuid>,
    started_at: Option<NaiveDateTime>,
    finished_at: Option<NaiveDateTime>,
    error_message: Option<&str>,
    result_json: &JsonValue,
) -> Result<Option<AutomationAction>, anyhow::Error> {
    let existing = fetch_action_for_update_tx(tx, id).await?;
    let Some(existing) = existing else {
        return Ok(None);
    };
    if matches!(
        existing.status.as_str(),
        action_status::SUCCEEDED | action_status::FAILED | action_status::PARTIAL_FAILURE
    ) {
        return Ok(None);
    }
    let row = sqlx::query_as::<_, AutomationAction>(
        r#"
        UPDATE automation_actions
        SET status        = $1,
            node_action_id = COALESCE($2, node_action_id),
            started_at    = COALESCE($3, started_at),
            finished_at   = COALESCE($4, finished_at),
            error         = $5,
            result_json   = $6,
            leased_until  = NULL,
            updated_at    = now()
        WHERE id = $7
        RETURNING *
        "#,
    )
    .bind(new_status)
    .bind(node_action_id)
    .bind(started_at)
    .bind(finished_at)
    .bind(error_message)
    .bind(result_json)
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(Some(row))
}

/// Fetch a run with a row-lock for the result-callback path.
pub async fn fetch_run_for_update_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> Result<Option<AutomationRun>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationRun>(
        "SELECT * FROM automation_runs WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

/// Apply a Node-reported success: bumps `actions_today`, sets position
/// state, clears `consecutive_failures`.
pub async fn apply_run_success_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    new_current_asset: Option<&str>,
    new_current_legs: Option<&JsonValue>,
    clear_position: bool,
    new_status: Option<&str>,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE automation_runs
        SET current_asset = CASE WHEN $4 THEN NULL ELSE COALESCE($1, current_asset) END,
            current_legs  = CASE WHEN $4 THEN NULL ELSE COALESCE($2, current_legs) END,
            actions_today = CASE
                WHEN actions_today_reset_at < now() - INTERVAL '1 day' THEN 1
                ELSE actions_today + 1
            END,
            actions_today_reset_at = CASE
                WHEN actions_today_reset_at < now() - INTERVAL '1 day' THEN now()
                ELSE actions_today_reset_at
            END,
            last_action_at = now(),
            consecutive_failures = 0,
            status = COALESCE($5, status),
            updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(new_current_asset)
    .bind(new_current_legs)
    .bind(id)
    .bind(clear_position)
    .bind(new_status)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Apply a Node-reported failure. Bumps `consecutive_failures` and may
/// transition the run to FAILED if the threshold is crossed.
pub async fn apply_run_failure_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    error: &str,
    max_consecutive_failures: i32,
) -> Result<i32, anyhow::Error> {
    let row: (i32,) = sqlx::query_as(
        r#"
        UPDATE automation_runs
        SET last_error = $1,
            last_error_at = now(),
            consecutive_failures = consecutive_failures + 1,
            status = CASE
                WHEN consecutive_failures + 1 >= $2 THEN 'FAILED'
                ELSE status
            END,
            updated_at = now()
        WHERE id = $3
        RETURNING consecutive_failures
        "#,
    )
    .bind(error)
    .bind(max_consecutive_failures)
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.0)
}

pub async fn list_automation_actions(
    db: Arc<PgPool>,
    run_id: Uuid,
) -> Result<Vec<AutomationAction>, anyhow::Error> {
    let rows = sqlx::query_as::<_, AutomationAction>(
        "SELECT * FROM automation_actions WHERE run_id = $1 ORDER BY created_at DESC",
    )
    .bind(run_id)
    .fetch_all(&*db)
    .await?;
    Ok(rows)
}

pub async fn get_automation_action(
    db: Arc<PgPool>,
    id: Uuid,
) -> Result<Option<AutomationAction>, anyhow::Error> {
    let row = sqlx::query_as::<_, AutomationAction>(
        "SELECT * FROM automation_actions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&*db)
    .await?;
    Ok(row)
}
