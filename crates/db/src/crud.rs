use std::sync::Arc;

use sqlx::PgPool;

use crate::types::FundingRate;

pub async fn insert_funding_rate(
    db_conn: Arc<PgPool>,
    funding_rate: FundingRate,
) -> Result<(), anyhow::Error> {
    let query = r#"
        INSERT INTO funding_rate (id, platform, symbol, rate, mark_px, timestamp, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    "#;

    sqlx::query(query)
        .bind(funding_rate.id)
        .bind(funding_rate.platform)
        .bind(funding_rate.symbol)
        .bind(funding_rate.rate)
        .bind(funding_rate.mark_px)
        .bind(funding_rate.timestamp)
        .bind(funding_rate.created_at)
        .bind(funding_rate.updated_at)
        .execute(&*db_conn)
        .await?;

    Ok(())
}
