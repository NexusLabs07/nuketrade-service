use std::sync::Arc;

use sqlx::PgPool;

use crate::types::{FundingRate, Points, User, Wallet};

pub async fn insert_funding_rate(
    db_conn: Arc<PgPool>,
    funding_rate: FundingRate,
) -> Result<uuid::Uuid, anyhow::Error> {
    let query = r#"
        INSERT INTO funding_rate (id, platform, symbol, rate, mark_px, timestamp)
        VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    sqlx::query(query)
        .bind(funding_rate.id)
        .bind(funding_rate.platform)
        .bind(funding_rate.symbol)
        .bind(funding_rate.rate)
        .bind(funding_rate.mark_px)
        .bind(funding_rate.timestamp)
        .execute(&*db_conn)
        .await?;

    Ok(funding_rate.id)
}

pub async fn insert_wallet(
    db_conn: Arc<PgPool>,
    wallet: Wallet,
) -> Result<uuid::Uuid, anyhow::Error> {
    let query = r#"
        INSERT INTO wallets(id, turnkey_evm_address)
        VALUES($1, $2)
    "#;

    sqlx::query(query)
        .bind(wallet.id)
        .bind(wallet.turnkey_evm_address)
        .execute(&*db_conn)
        .await?;

    Ok(wallet.id)
}

pub async fn insert_user(db_conn: Arc<PgPool>, user: User) -> Result<uuid::Uuid, anyhow::Error> {
    let query = r#"
            INSERT INTO users(id, email, connected_evm_address, connected_solana_address, referral_code, referred_by, wallet_id)
            VALUES($1, $2, $3, $4, $5, $6, $7)
            "#;

    sqlx::query(query)
        .bind(user.id)
        .bind(user.email)
        .bind(user.connected_evm_address)
        .bind(user.connected_solana_address)
        .bind(user.referral_code)
        .bind(user.referred_by)
        .bind(user.wallet_id)
        .execute(&*db_conn)
        .await?;

    Ok(user.id)
}

pub async fn insert_points(
    db_conn: Arc<PgPool>,
    points: Points,
) -> Result<uuid::Uuid, anyhow::Error> {
    let query = r#"
        INSERT INTO points(id, user_id, total_points)
        VALUES($1, #2, $3)
    "#;

    sqlx::query(query)
        .bind(points.id)
        .bind(points.user_id)
        .bind(points.points_to_added)
        .execute(&*db_conn)
        .await?;

    Ok(points.id)
}
