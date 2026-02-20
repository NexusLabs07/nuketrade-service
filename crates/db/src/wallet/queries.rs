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
