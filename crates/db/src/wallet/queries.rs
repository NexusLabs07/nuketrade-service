use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

use crate::wallet::models::{HlAgentWallet, Wallet};

pub async fn insert_wallet(
    db_conn: Arc<PgPool>,
    wallet: Wallet,
) -> Result<uuid::Uuid, anyhow::Error> {
    insert_wallet_with_executor(&*db_conn, &wallet).await
}

pub async fn insert_wallet_with_executor<'e, E>(
    executor: E,
    wallet: &Wallet,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO wallets(id, turnkey_evm_address, turnkey_solana_address)
        VALUES($1, $2, $3)
    "#;

    sqlx::query(query)
        .bind(wallet.id)
        .bind(&wallet.turnkey_evm_address)
        .bind(&wallet.turnkey_solana_address)
        .execute(executor)
        .await?;

    Ok(wallet.id)
}

pub async fn upsert_wallet_with_executor<'e, E>(
    executor: E,
    wallet: &Wallet,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO wallets(id, turnkey_evm_address, turnkey_solana_address)
        VALUES($1, $2, $3)
        ON CONFLICT (turnkey_evm_address) DO UPDATE
        SET turnkey_solana_address = EXCLUDED.turnkey_solana_address,
            updated_at = now()
        RETURNING id
    "#;

    let (id,): (uuid::Uuid,) = sqlx::query_as(query)
        .bind(wallet.id)
        .bind(&wallet.turnkey_evm_address)
        .bind(&wallet.turnkey_solana_address)
        .fetch_one(executor)
        .await?;

    Ok(id)
}

/// Look up a user's Hyperliquid agent wallet. Returns `Ok(None)` if the
/// user hasn't been provisioned yet — the worker treats that as a hard
/// failure for the intent.
pub async fn get_hl_agent_for_user(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<Option<HlAgentWallet>, anyhow::Error> {
    let query = r#"
        SELECT user_id, turnkey_suborg_id, turnkey_wallet_id, evm_address,
               approved_on_hl, created_at, updated_at
        FROM hl_agent_wallets
        WHERE user_id = $1
    "#;

    let row: Option<HlAgentWallet> = sqlx::query_as(query)
        .bind(user_id)
        .fetch_optional(&*db_conn)
        .await?;

    Ok(row)
}
