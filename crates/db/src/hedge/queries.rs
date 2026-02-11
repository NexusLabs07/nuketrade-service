use std::sync::Arc;

use sqlx::PgPool;

use crate::hedge::models::{
    HedgeIntent, HedgeLeg, NewHedgeIntent, NewHedgeLeg, NewTxReference, TxReference,
};

// ============================= CRUD =============================
/// Create a hedge intent along with its two legs in a single transaction.
pub async fn create_hedge_intent_with_legs(
    db: Arc<PgPool>,
    intent: &NewHedgeIntent,
    legs: &[NewHedgeLeg],
) -> Result<uuid::Uuid, anyhow::Error> {
    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO hedge_intents (id, user_id, asset, exchange_a, exchange_b, margin_usd, leverage, evm_address, solana_address, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'CREATED')
        "#,
    )
    .bind(intent.id)
    .bind(intent.user_id)
    .bind(&intent.asset)
    .bind(&intent.exchange_a)
    .bind(&intent.exchange_b)
    .bind(intent.margin_usd)
    .bind(intent.leverage)
    .bind(&intent.evm_address)
    .bind(&intent.solana_address)
    .execute(&mut *tx)
    .await?;

    for leg in legs {
        sqlx::query(
            r#"
            INSERT INTO hedge_legs (id, hedge_intent_id, exchange, chain, target_amount_usd, funded_amount_usd, status)
            VALUES ($1, $2, $3, $4, $5, 0, 'PENDING')
            "#,
        )
        .bind(leg.id)
        .bind(leg.hedge_intent_id)
        .bind(&leg.exchange)
        .bind(leg.chain)
        .bind(leg.target_amount_usd)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(intent.id)
}

/// Get a hedge intent by its ID.
pub async fn get_hedge_intent(
    db: Arc<PgPool>,
    id: uuid::Uuid,
) -> Result<Option<HedgeIntent>, anyhow::Error> {
    let intent = sqlx::query_as::<_, HedgeIntent>("SELECT * FROM hedge_intents WHERE id = $1")
        .bind(id)
        .fetch_optional(&*db)
        .await?;

    Ok(intent)
}

/// Get all hedge intents for a user.
pub async fn get_hedge_intents_by_user(
    db: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<Vec<HedgeIntent>, anyhow::Error> {
    let intents = sqlx::query_as::<_, HedgeIntent>(
        "SELECT * FROM hedge_intents WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(&*db)
    .await?;

    Ok(intents)
}

/// Get all legs for a hedge intent.
pub async fn get_hedge_legs(
    db: Arc<PgPool>,
    hedge_intent_id: uuid::Uuid,
) -> Result<Vec<HedgeLeg>, anyhow::Error> {
    let legs = sqlx::query_as::<_, HedgeLeg>(
        "SELECT * FROM hedge_legs WHERE hedge_intent_id = $1 ORDER BY created_at ASC",
    )
    .bind(hedge_intent_id)
    .fetch_all(&*db)
    .await?;

    Ok(legs)
}

/// Update the status of a hedge intent (also bumps updated_at).
pub async fn update_hedge_intent_status(
    db: Arc<PgPool>,
    id: uuid::Uuid,
    status: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query("UPDATE hedge_intents SET status = $1, updated_at = now() WHERE id = $2")
        .bind(status)
        .bind(id)
        .execute(&*db)
        .await?;

    Ok(())
}

/// Update the status of a hedge leg (also bumps updated_at).
pub async fn update_hedge_leg_status(
    db: Arc<PgPool>,
    leg_id: uuid::Uuid,
    status: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query("UPDATE hedge_legs SET status = $1, updated_at = now() WHERE id = $2")
        .bind(status)
        .bind(leg_id)
        .execute(&*db)
        .await?;

    Ok(())
}

/// Update a leg's funded amount after a successful deposit.
pub async fn update_hedge_leg_funded_amount(
    db: Arc<PgPool>,
    leg_id: uuid::Uuid,
    funded_amount_usd: f64,
) -> Result<(), anyhow::Error> {
    sqlx::query("UPDATE hedge_legs SET funded_amount_usd = $1, updated_at = now() WHERE id = $2")
        .bind(funded_amount_usd)
        .bind(leg_id)
        .execute(&*db)
        .await?;

    Ok(())
}

/// Increment the retry count on a leg and optionally record the last error.
pub async fn increment_hedge_leg_retry(
    db: Arc<PgPool>,
    leg_id: uuid::Uuid,
    error: Option<&str>,
) -> Result<i16, anyhow::Error> {
    let row: (i16,) = sqlx::query_as(
        r#"
        UPDATE hedge_legs
        SET retry_count = retry_count + 1,
            last_error = COALESCE($1, last_error),
            updated_at = now()
        WHERE id = $2
        RETURNING retry_count
        "#,
    )
    .bind(error)
    .bind(leg_id)
    .fetch_one(&*db)
    .await?;

    Ok(row.0)
}

/// Insert a transaction reference for a hedge leg.
pub async fn insert_tx_reference(
    db: Arc<PgPool>,
    tx_ref: &NewTxReference,
) -> Result<uuid::Uuid, anyhow::Error> {
    sqlx::query(
        r#"
        INSERT INTO tx_references (id, hedge_leg_id, action, tx_hash, chain, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tx_ref.id)
    .bind(tx_ref.hedge_leg_id)
    .bind(&tx_ref.action)
    .bind(&tx_ref.tx_hash)
    .bind(&tx_ref.chain)
    .bind(&tx_ref.status)
    .execute(&*db)
    .await?;

    Ok(tx_ref.id)
}

/// Get all tx references for a leg.
pub async fn get_tx_references(
    db: Arc<PgPool>,
    hedge_leg_id: uuid::Uuid,
) -> Result<Vec<TxReference>, anyhow::Error> {
    let refs = sqlx::query_as::<_, TxReference>(
        "SELECT * FROM tx_references WHERE hedge_leg_id = $1 ORDER BY created_at ASC",
    )
    .bind(hedge_leg_id)
    .fetch_all(&*db)
    .await?;

    Ok(refs)
}

/// Update a leg's existing balance fields after querying on-chain/protocol balances.
pub async fn update_hedge_leg_existing_balances(
    db: Arc<PgPool>,
    leg_id: uuid::Uuid,
    existing_margin_usd: f64,
    existing_onchain_usd: f64,
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE hedge_legs
        SET existing_margin_usd = $1,
            existing_onchain_usd = $2,
            updated_at = now()
        WHERE id = $3
        "#,
    )
    .bind(existing_margin_usd)
    .bind(existing_onchain_usd)
    .bind(leg_id)
    .execute(&*db)
    .await?;

    Ok(())
}

/// Check if a tx reference already exists for an action on a leg (idempotency guard).
pub async fn tx_reference_exists(
    db: Arc<PgPool>,
    hedge_leg_id: uuid::Uuid,
    action: &str,
    status: &str,
) -> Result<bool, anyhow::Error> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM tx_references WHERE hedge_leg_id = $1 AND action = $2 AND status = $3)",
    )
    .bind(hedge_leg_id)
    .bind(action)
    .bind(status)
    .fetch_one(&*db)
    .await?;

    Ok(row.0)
}
