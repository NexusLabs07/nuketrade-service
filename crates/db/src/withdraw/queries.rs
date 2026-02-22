use std::sync::Arc;

use sqlx::PgPool;

use crate::withdraw::models::{
    NewWithdrawalIntent, NewWithdrawalStep, WithdrawalIntent, WithdrawalStep,
};

const VALID_INTENT_STATUSES: &[&str] =
    &["CREATED", "WITHDRAWING", "WITHDRAWN", "BRIDGING", "COMPLETED", "FAILED"];

const VALID_STEP_STATUSES: &[&str] = &["PENDING", "CONFIRMED", "FAILED"];

pub async fn create_withdrawal_intent(
    db: Arc<PgPool>,
    intent: &NewWithdrawalIntent,
) -> Result<uuid::Uuid, anyhow::Error> {
    sqlx::query(
        r#"
        INSERT INTO withdrawal_intents
            (id, user_id, exchange, amount_usd, evm_address, recipient, destination_chain_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'CREATED')
        "#,
    )
    .bind(intent.id)
    .bind(intent.user_id)
    .bind(&intent.exchange)
    .bind(intent.amount_usd)
    .bind(&intent.evm_address)
    .bind(&intent.recipient)
    .bind(intent.destination_chain_id)
    .execute(&*db)
    .await?;

    Ok(intent.id)
}

pub async fn get_withdrawal_intent(
    db: Arc<PgPool>,
    id: uuid::Uuid,
) -> Result<Option<WithdrawalIntent>, anyhow::Error> {
    let intent =
        sqlx::query_as::<_, WithdrawalIntent>("SELECT * FROM withdrawal_intents WHERE id = $1")
            .bind(id)
            .fetch_optional(&*db)
            .await?;

    Ok(intent)
}

pub async fn get_withdrawal_intents_by_user(
    db: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<Vec<WithdrawalIntent>, anyhow::Error> {
    let intents = sqlx::query_as::<_, WithdrawalIntent>(
        "SELECT * FROM withdrawal_intents WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&*db)
    .await?;

    Ok(intents)
}

pub async fn update_withdrawal_intent_status(
    db: Arc<PgPool>,
    id: uuid::Uuid,
    status: &str,
) -> Result<(), anyhow::Error> {
    if !VALID_INTENT_STATUSES.contains(&status) {
        anyhow::bail!("invalid intent status '{status}': must be one of {VALID_INTENT_STATUSES:?}");
    }

    sqlx::query(
        "UPDATE withdrawal_intents SET status = $1, updated_at = now() WHERE id = $2",
    )
    .bind(status)
    .bind(id)
    .execute(&*db)
    .await?;

    Ok(())
}

pub async fn increment_withdrawal_intent_retry(
    db: Arc<PgPool>,
    id: uuid::Uuid,
    error: Option<&str>,
) -> Result<i16, anyhow::Error> {
    let row: (i16,) = sqlx::query_as(
        r#"
        UPDATE withdrawal_intents
        SET retry_count = retry_count + 1,
            last_error  = COALESCE($1, last_error),
            updated_at  = now()
        WHERE id = $2
        RETURNING retry_count
        "#,
    )
    .bind(error)
    .bind(id)
    .fetch_one(&*db)
    .await?;

    Ok(row.0)
}

pub async fn insert_withdrawal_step(
    db: Arc<PgPool>,
    step: &NewWithdrawalStep,
) -> Result<uuid::Uuid, anyhow::Error> {
    sqlx::query(
        r#"
        INSERT INTO withdrawal_steps (id, withdrawal_intent_id, step, chain_id, status)
        VALUES ($1, $2, $3, $4, 'PENDING')
        "#,
    )
    .bind(step.id)
    .bind(step.withdrawal_intent_id)
    .bind(&step.step)
    .bind(step.chain_id)
    .execute(&*db)
    .await?;

    Ok(step.id)
}

pub async fn update_withdrawal_step_status(
    db: Arc<PgPool>,
    step_id: uuid::Uuid,
    tx_hash: Option<&str>,
    status: &str,
) -> Result<(), anyhow::Error> {
    if !VALID_STEP_STATUSES.contains(&status) {
        anyhow::bail!("invalid step status '{status}': must be one of {VALID_STEP_STATUSES:?}");
    }

    sqlx::query(
        "UPDATE withdrawal_steps SET tx_hash = COALESCE($1, tx_hash), status = $2, updated_at = now() WHERE id = $3",
    )
    .bind(tx_hash)
    .bind(status)
    .bind(step_id)
    .execute(&*db)
    .await?;

    Ok(())
}

pub async fn get_withdrawal_steps(
    db: Arc<PgPool>,
    withdrawal_intent_id: uuid::Uuid,
) -> Result<Vec<WithdrawalStep>, anyhow::Error> {
    let steps = sqlx::query_as::<_, WithdrawalStep>(
        "SELECT * FROM withdrawal_steps WHERE withdrawal_intent_id = $1 ORDER BY created_at ASC",
    )
    .bind(withdrawal_intent_id)
    .fetch_all(&*db)
    .await?;

    Ok(steps)
}

/// Fetch the most recent PENDING step for a given step name (used in action-result handling).
pub async fn get_pending_step(
    db: Arc<PgPool>,
    withdrawal_intent_id: uuid::Uuid,
    step_name: &str,
) -> Result<Option<WithdrawalStep>, anyhow::Error> {
    let step = sqlx::query_as::<_, WithdrawalStep>(
        r#"
        SELECT * FROM withdrawal_steps
        WHERE withdrawal_intent_id = $1
          AND step = $2
          AND status = 'PENDING'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(withdrawal_intent_id)
    .bind(step_name)
    .fetch_optional(&*db)
    .await?;

    Ok(step)
}
