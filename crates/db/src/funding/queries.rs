use std::sync::Arc;

use sqlx::PgPool;

use crate::funding::{AverageFundingStats, HourlyFundingRate, models::FundingRate};

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

pub async fn get_token_chart_info(
    db_conn: Arc<PgPool>,
    symbol: String,
    timeframe: String,
) -> Result<Vec<FundingRate>, anyhow::Error> {
    let query = match timeframe.as_str() {
        "30m" => {
            r#"SELECT id, platform, symbol, rate, mark_px, timestamp FROM funding_rate WHERE symbol = $1 AND timestamp >= NOW() - INTERVAL '7 days' ORDER BY timestamp ASC"#
        }
        "1h" => {
            r#"
            SELECT DISTINCT ON (platform, ts_hour)
            id,
            platform,
            symbol,
            rate,
            mark_px,
            timestamp
            FROM funding_rate
            WHERE symbol = $1 AND timestamp >= NOW() - INTERVAL '7 days'
            ORDER BY platform, ts_hour, timestamp DESC
            "#
        }
        "24h" => {
            r#"
            SELECT DISTINCT ON (platform, ts_day)
            id,
            platform,
            symbol,
            rate,
            mark_px,
            timestamp
            FROM funding_rate
            WHERE symbol = $1 AND timestamp >= NOW() - INTERVAL '30 days'
            ORDER BY platform, ts_day, timestamp DESC
            "#
        }
        "7d" => {
            r#"
            SELECT DISTINCT ON (platform, ts_week)
            id,
            platform,
            symbol,
            rate,
            mark_px,
            timestamp
            FROM funding_rate
            WHERE symbol = $1 AND timestamp >= NOW() - INTERVAL '3 months'
            ORDER BY platform, ts_week, timestamp DESC
            "#
        }
        "30d" => {
            r#"
            SELECT DISTINCT ON (platform, ts_month)
            id,
            platform,
            symbol,
            rate,
            mark_px,
            timestamp
            FROM funding_rate
            WHERE symbol = $1
            ORDER BY platform, ts_month, timestamp DESC
            "#
        }
        _ => {
            log::warn!("Invalid timeframe: {timeframe}, continuing with default timeframe");
            r#"SELECT id, platform, symbol, rate, mark_px, timestamp FROM funding_rate WHERE symbol = $1 ORDER BY timestamp ASC"#
        }
    };

    let rows = sqlx::query_as::<_, FundingRate>(query)
        .bind(symbol)
        .fetch_all(&*db_conn)
        .await?;

    Ok(rows)
}

pub async fn get_7d_funding_stats(
    db: Arc<PgPool>,
) -> Result<Vec<AverageFundingStats>, anyhow::Error> {
    let query = r#"
        SELECT
            symbol,
            platform,
            MAX(rate) AS max_rate,
            MIN(rate) AS min_rate,
            AVG(rate) AS avg_rate
        FROM funding_rate
        WHERE timestamp >= NOW() - INTERVAL '7 days'
        GROUP BY symbol, platform
    "#;

    let rows = sqlx::query_as::<_, AverageFundingStats>(query)
        .fetch_all(&*db)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to query 7d funding stats: {e}");
            vec![]
        });

    Ok(rows)
}

pub async fn get_7d_hourly_rates(db: Arc<PgPool>) -> Result<Vec<HourlyFundingRate>, anyhow::Error> {
    let query = r#"
        SELECT symbol, platform, ts_hour, AVG(rate) AS rate
        FROM funding_rate
        WHERE timestamp >= NOW() - INTERVAL '7 days'
        GROUP BY symbol, platform, ts_hour
        ORDER BY symbol, ts_hour
    "#;

    let rows = sqlx::query_as::<_, HourlyFundingRate>(query)
        .fetch_all(&*db)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to query 7d hourly rates: {e}");
            vec![]
        });

    Ok(rows)
}
