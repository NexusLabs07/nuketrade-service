use std::sync::Arc;

use sqlx::{Executor, PgPool, Postgres};

use crate::{
    points::{
        models::Points,
        queries::{insert_points_with_executor, update_points_with_executor},
    },
    user::models::User,
    wallet::{models::Wallet, queries::insert_wallet_with_executor},
};

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
        .bind(user.referred_by)
        .bind(user.wallet_id)
        .execute(executor)
        .await?;

    Ok(user.id)
}

pub async fn insert_user_with_wallet_and_points(
    db_conn: Arc<PgPool>,
    wallet: Wallet,
    user: User,
    points: Points,
    referred_by_user: Option<User>,
) -> Result<uuid::Uuid, anyhow::Error> {
    let mut tx = db_conn.begin().await?;

    if let Some(referred_user) = referred_by_user {
        update_points_with_executor(&mut *tx, referred_user.id, 25).await?;
    }

    insert_wallet_with_executor(&mut *tx, &wallet).await?;
    insert_user_with_executor(&mut *tx, &user).await?;
    insert_points_with_executor(&mut *tx, &points).await?;

    tx.commit().await?;

    Ok(user.id)
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

pub async fn insert_user(db_conn: Arc<PgPool>, user: User) -> Result<uuid::Uuid, anyhow::Error> {
    insert_user_with_executor(&*db_conn, &user).await
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
