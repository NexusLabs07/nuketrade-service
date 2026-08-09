use std::{collections::HashMap, env, sync::Arc, time::Duration};

use chrono::Utc;
use db::funding::{FundingRate, insert_funding_rates};
use perp_core::{MarketFeedUpdate, exchange::PerpetualExchange};
use sqlx::PgPool;
use tokio::{
    sync::mpsc,
    time::{Instant, MissedTickBehavior},
};
use uuid::Uuid;

use crate::RiseXClient;

const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 5;
const MINIMUM_POLL_INTERVAL_SECONDS: u64 = 1;
const MAXIMUM_POLL_INTERVAL_SECONDS: u64 = 300;
const DATABASE_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(30 * 60);

fn poll_interval() -> Duration {
    let configured = env::var("RISEX_POLL_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| match value.parse::<u64>() {
            Ok(value) => Some(value),
            Err(err) => {
                log::warn!("Ignoring invalid RISEX_POLL_INTERVAL_SECONDS value: {err}");
                None
            }
        })
        .unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS)
        .clamp(MINIMUM_POLL_INTERVAL_SECONDS, MAXIMUM_POLL_INTERVAL_SECONDS);

    Duration::from_secs(configured)
}

fn funding_rows(snapshot: &HashMap<String, (f64, f64)>) -> Vec<FundingRate> {
    let timestamp = Utc::now().naive_utc();

    snapshot
        .iter()
        .map(|(symbol, (mark_price, hourly_funding_rate))| FundingRate {
            id: Uuid::new_v4(),
            platform: PerpetualExchange::RiseX.to_string(),
            symbol: symbol.clone(),
            rate: *hourly_funding_rate,
            mark_px: *mark_price,
            timestamp,
        })
        .collect()
}

/// Poll the public RiseX markets endpoint and publish validated mark/funding
/// snapshots into Nuketrade's existing feed manager.
///
/// The function retains the last good snapshot during an upstream failure by
/// not publishing empty or invalid responses. Database persistence follows the
/// same thirty-minute cadence as the existing venue feeds.
pub async fn start_risex_funding_feed(
    client: RiseXClient,
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let mut poll_tick = tokio::time::interval(poll_interval());
    poll_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut last_database_attempt: Option<Instant> = None;

    loop {
        poll_tick.tick().await;

        let snapshot = match client.fetch_feed_snapshot().await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                log::warn!("RiseX market polling failed; retaining last good snapshot: {err:#}");
                continue;
            }
        };

        let database_write_due = last_database_attempt
            .is_none_or(|attempt| attempt.elapsed() >= DATABASE_SNAPSHOT_INTERVAL);

        if database_write_due {
            last_database_attempt = Some(Instant::now());

            let rows = funding_rows(&snapshot);
            let db = db_conn.clone();

            tokio::spawn(async move {
                if let Err(err) = insert_funding_rates(db, rows).await {
                    log::warn!("RiseX funding-rate database insert failed: {err:#}");
                }
            });
        }

        let update = MarketFeedUpdate {
            exchange: PerpetualExchange::RiseX,
            data: snapshot,
        };

        if let Err(err) = feed_tx.send(update).await {
            log::error!("RiseX feed manager receiver dropped: {err}");
            return;
        }
    }
}
