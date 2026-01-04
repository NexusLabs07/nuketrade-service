use std::sync::Arc;

use sqlx::PgPool;

use crate::types::FundingRate;

pub async fn insert_funding_rate(
    db_conn: Arc<PgPool>,
    funding_rate: FundingRate,
) -> Result<(), anyhow::Error> {
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

    Ok(())
}
