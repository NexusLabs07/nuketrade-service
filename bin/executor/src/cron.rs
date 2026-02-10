use std::{collections::HashMap, sync::Arc};

use db::funding::{get_7d_funding_stats, get_7d_hourly_rates};
use sqlx::PgPool;
use tokio::sync::watch;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{PairSpread, SevenDayApr};

async fn compute_seven_day_apr(db: Arc<PgPool>) -> Option<SevenDayApr> {
    let funding_stats = match get_7d_funding_stats(db.clone()).await {
        Ok(stats) => stats,
        Err(e) => {
            log::error!("Failed to fetch 7d funding stats: {e}");
            return None;
        }
    };

    // Build avg_apr: symbol -> platform -> avg_rate
    let mut seven_day_avg_apr: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for funding in funding_stats {
        seven_day_avg_apr
            .entry(funding.symbol)
            .or_default()
            .insert(funding.platform, funding.avg_rate * 100.0);
    }

    let hourly_rates = match get_7d_hourly_rates(db).await {
        Ok(rates) => rates,
        Err(e) => {
            log::error!("Failed to fetch 7d hourly rates: {e}");
            return None;
        }
    };

    // (BTC, 9 FEB 3PM) -> {Hyperliquid -> 0.01}
    // Group hourly rates by (symbol, ts_hour) -> platform -> rate
    let mut hourly_grouped: HashMap<(String, chrono::NaiveDateTime), HashMap<String, f64>> =
        HashMap::new();
    for rate in hourly_rates {
        hourly_grouped
            .entry((rate.symbol, rate.ts_hour))
            .or_default()
            .insert(rate.platform, rate.rate);
    }

    // For each (symbol, hour), compute pairwise spreads and accumulate.
    // short_p = platform with higher rate (you receive funding by shorting)
    // long_p  = platform with lower rate  (you pay less funding by longing)
    let mut spread_acc: HashMap<(String, String, String), f64> = HashMap::new();
    for ((symbol, _ts_hour), platforms) in &hourly_grouped {
        let platform_list: Vec<&String> = platforms.keys().collect();
        for i in 0..platform_list.len() {
            for j in (i + 1)..platform_list.len() {
                let p_a = platform_list[i];
                let p_b = platform_list[j];

                let (short_p, long_p) = if platforms[p_a] >= platforms[p_b] {
                    (p_a, p_b)
                } else {
                    (p_b, p_a)
                };

                // always positive: rate received (short) - rate paid (long)
                let spread = (platforms[short_p] - platforms[long_p]) * 100.0;
                *spread_acc
                    .entry((symbol.clone(), long_p.clone(), short_p.clone()))
                    .or_default() += spread;
            }
        }
    }

    // Convert to spread_apr: symbol -> Vec<PairSpread>
    let mut seven_day_spread_apr: HashMap<String, Vec<PairSpread>> = HashMap::new();
    for ((symbol, long_platform, short_platform), total_spread) in spread_acc {
        seven_day_spread_apr
            .entry(symbol)
            .or_default()
            .push(PairSpread {
                long_platform,
                short_platform,
                total_spread,
            });
    }

    Some(SevenDayApr {
        seven_day_avg_apr,
        seven_day_spread_apr,
    })
}

pub async fn calculate_best_pair(
    db: Arc<PgPool>,
    scheduler: JobScheduler,
    tx: watch::Sender<SevenDayApr>,
) -> Result<(), anyhow::Error> {
    // Run immediately on startup
    if let Some(apr) = compute_seven_day_apr(db.clone()).await {
        if let Err(e) = tx.send(apr) {
            log::error!("Failed to send initial seven_day_apr: {e}");
        }
    }

    // Then repeat every 24 hours: "0 0 0 * * *" = midnight daily
    let job = Job::new_async("0 0 0 * * *", move |_uuid, _lock| {
        let db_clone = db.clone();
        let tx = tx.clone();
        Box::pin(async move {
            if let Some(apr) = compute_seven_day_apr(db_clone).await {
                if let Err(e) = tx.send(apr) {
                    log::error!("Failed to send seven_day_apr update: {e}");
                }
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    Ok(())
}
