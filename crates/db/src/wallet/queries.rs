use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

use crate::wallet::models::Wallet;

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
