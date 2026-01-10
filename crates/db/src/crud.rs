use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

use crate::types::{FundingRate, Points, User, Wallet};

async fn insert_wallet_with_executor<'e, E>(
    executor: E,
    wallet: &Wallet,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO wallets(id, turnkey_evm_address)
        VALUES($1, $2)
    "#;

    sqlx::query(query)
        .bind(wallet.id)
        .bind(&wallet.turnkey_evm_address)
        .execute(executor)
        .await?;

    Ok(wallet.id)
}

async fn insert_user_with_executor<'e, E>(
    executor: E,
    user: &User,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO users(id, email, connected_evm_address, connected_solana_address, referral_code, referred_by, wallet_id)
        VALUES($1, $2, $3, $4, $5, $6, $7)
    "#;

    sqlx::query(query)
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.connected_evm_address)
        .bind(&user.connected_solana_address)
        .bind(&user.referral_code)
        .bind(&user.referred_by)
        .bind(user.wallet_id)
        .execute(executor)
        .await?;

    Ok(user.id)
}

async fn insert_points_with_executor<'e, E>(
    executor: E,
    points: &Points,
) -> Result<uuid::Uuid, anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        INSERT INTO points(id, user_id, total_points)
        VALUES($1, $2, $3)
    "#;

    sqlx::query(query)
        .bind(points.id)
        .bind(points.user_id)
        .bind(points.points_to_add)
        .execute(executor)
        .await?;

    Ok(points.id)
}

pub async fn update_points_with_executor<'e, E>(
    executor: E,
    user_id: uuid::Uuid,
    points: i32,
) -> Result<(), anyhow::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let query = r#"
        UPDATE points
        SET total_points = total_points + $1
        WHERE user_id = $2
    "#;

    sqlx::query(query)
        .bind(points)
        .bind(user_id)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn insert_wallet(
    db_conn: Arc<PgPool>,
    wallet: Wallet,
) -> Result<uuid::Uuid, anyhow::Error> {
    insert_wallet_with_executor(&*db_conn, &wallet).await
}

pub async fn insert_user(db_conn: Arc<PgPool>, user: User) -> Result<uuid::Uuid, anyhow::Error> {
    insert_user_with_executor(&*db_conn, &user).await
}

pub async fn insert_points(
    db_conn: Arc<PgPool>,
    points: Points,
) -> Result<uuid::Uuid, anyhow::Error> {
    insert_points_with_executor(&*db_conn, &points).await
}

pub async fn insert_user_with_wallet_and_points(
    db_conn: Arc<PgPool>,
    wallet: Wallet,
    user: User,
    points: Points,
    referred_by_user: Option<User>,
) -> Result<uuid::Uuid, anyhow::Error> {
    let mut tx = db_conn.begin().await?;

    let referred_by_user_id = match referred_by_user {
        Some(user) => Some(user.id),
        None => None,
    };

    if referred_by_user_id.is_some() {
        update_points_with_executor(&mut *tx, referred_by_user_id.unwrap(), 25).await?;
    }

    insert_wallet_with_executor(&mut *tx, &wallet).await?;
    insert_user_with_executor(&mut *tx, &user).await?;
    insert_points_with_executor(&mut *tx, &points).await?;

    tx.commit().await?;

    Ok(user.id)
}

pub async fn update_points(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
    points: i32,
) -> Result<(), anyhow::Error> {
    update_points_with_executor(&*db_conn, user_id, points).await
}

pub async fn get_user_from_referral_code(
    db_conn: Arc<PgPool>,
    referral_code: String,
) -> Result<User, anyhow::Error> {
    let query = "SELECT * FROM users WHERE referral_code = $1";

    let user = match sqlx::query_as::<_, User>(query)
        .bind(&referral_code)
        .fetch_one(&*db_conn)
        .await
    {
        Ok(user) => user,
        Err(e) => {
            return Err(anyhow::Error::new(e));
        }
    };

    Ok(user)
}

pub async fn get_total_users(db_conn: Arc<PgPool>) -> Result<i64, anyhow::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}

pub async fn get_total_points(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<i32, anyhow::Error> {
    let row: (i32,) = sqlx::query_as("SELECT total_points FROM points WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}

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

pub async fn insert_funding_rates(
    db_conn: Arc<PgPool>,
    funding_rates: Vec<FundingRate>,
) -> Result<usize, anyhow::Error> {
    if funding_rates.is_empty() {
        return Ok(0);
    }

    let mut tx = db_conn.begin().await?;

    let query = r#"
        INSERT INTO funding_rate (id, platform, symbol, rate, mark_px, timestamp)
        VALUES ($1, $2, $3, $4, $5, $6)
    "#;

    let mut count = 0;
    for funding_rate in funding_rates {
        sqlx::query(query)
            .bind(funding_rate.id)
            .bind(funding_rate.platform)
            .bind(funding_rate.symbol)
            .bind(funding_rate.rate)
            .bind(funding_rate.mark_px)
            .bind(funding_rate.timestamp)
            .execute(&mut *tx)
            .await?;
        count += 1;
    }

    tx.commit().await?;

    Ok(count)
}

pub async fn get_referral_count_from_referral_code(
    db_conn: Arc<PgPool>,
    referral_code: String,
) -> Result<i32, anyhow::Error> {
    let query = r#"
        SELECT COUNT(*)::INT4 FROM users WHERE referred_by = (
            SELECT id FROM users WHERE referral_code = $1
        )
    "#;

    let row: (i32,) = sqlx::query_as(query)
        .bind(&referral_code)
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}

pub async fn get_referral_count_from_user_id(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<i32, anyhow::Error> {
    let query = "SELECT COUNT(*)::INT4 FROM users WHERE referred_by = $1";

    let row: (i32,) = sqlx::query_as(query)
        .bind(user_id)
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}

pub async fn get_user_position(
    db_conn: Arc<PgPool>,
    user_id: uuid::Uuid,
) -> Result<i64, anyhow::Error> {
    let query = r#"
        SELECT position FROM (
            SELECT id, ROW_NUMBER() OVER (ORDER BY created_at ASC) as position
            FROM users
        ) ranked
        WHERE id = $1
    "#;

    let row: (i64,) = sqlx::query_as(query)
        .bind(user_id)
        .fetch_one(&*db_conn)
        .await?;

    Ok(row.0)
}
