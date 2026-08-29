use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use db::funding::{get_7d_funding_stats, get_7d_hourly_rates};
use sqlx::PgPool;
use tokio::sync::watch;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{PairSpread, SevenDayApr};

const SEVEN_DAY_WINDOW_HOURS: usize = 168;

#[derive(Default)]
struct FixedPairWindow {
    total_spread: f64,
    observed_hours: BTreeSet<chrono::NaiveDateTime>,
}

/// Computes every directed long/short pair over one unchanged position direction.
///
/// Input rates must already be normalized to decimal funding per hour and grouped
/// by symbol and UTC hour. A directed pair is emitted only when both venues have
/// observations for every consecutive hour in the requested window; this prevents
/// incomplete data from being presented as a complete seven-day recommendation.
fn calculate_fixed_direction_spreads(
    hourly_grouped: &HashMap<(String, chrono::NaiveDateTime), HashMap<String, f64>>,
    required_hours: usize,
) -> HashMap<String, Vec<PairSpread>> {
    let mut windows: HashMap<(String, String, String), FixedPairWindow> = HashMap::new();

    for ((symbol, ts_hour), platforms) in hourly_grouped {
        let mut platform_list: Vec<&String> = platforms.keys().collect();
        platform_list.sort_unstable();

        for i in 0..platform_list.len() {
            for j in (i + 1)..platform_list.len() {
                let platform_a = platform_list[i];
                let platform_b = platform_list[j];
                let spread_a_long_b_short = (platforms[platform_b] - platforms[platform_a]) * 100.0;

                for (long_platform, short_platform, spread) in [
                    (platform_a, platform_b, spread_a_long_b_short),
                    (platform_b, platform_a, -spread_a_long_b_short),
                ] {
                    let window = windows
                        .entry((
                            symbol.clone(),
                            long_platform.clone(),
                            short_platform.clone(),
                        ))
                        .or_default();
                    window.total_spread += spread;
                    window.observed_hours.insert(*ts_hour);
                }
            }
        }
    }

    let mut spreads: HashMap<String, Vec<PairSpread>> = HashMap::new();
    for ((symbol, long_platform, short_platform), window) in windows {
        let complete = window.observed_hours.len() == required_hours
            && window
                .observed_hours
                .iter()
                .zip(window.observed_hours.iter().skip(1))
                .all(|(previous, next)| *next - *previous == chrono::Duration::hours(1));
        if !complete {
            continue;
        }

        spreads.entry(symbol).or_default().push(PairSpread {
            long_platform,
            short_platform,
            total_spread: window.total_spread,
        });
    }

    // Keep producer ordering deterministic and make the ranking contract
    // explicit even though downstream consumers defensively sort again.
    for pairs in spreads.values_mut() {
        pairs.sort_by(|a, b| {
            b.total_spread
                .total_cmp(&a.total_spread)
                .then_with(|| a.long_platform.cmp(&b.long_platform))
                .then_with(|| a.short_platform.cmp(&b.short_platform))
        });
    }

    spreads
}

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
        let avg_rate = if funding.platform == phoenix::helpers::funding::PLATFORM {
            phoenix::helpers::funding::normalize_stored_hourly_rate(funding.avg_rate)
        } else {
            funding.avg_rate
        };
        seven_day_avg_apr
            .entry(funding.symbol)
            .or_default()
            .insert(funding.platform, avg_rate * 100.0);
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
        let hourly = if rate.platform == phoenix::helpers::funding::PLATFORM {
            phoenix::helpers::funding::normalize_stored_hourly_rate(rate.rate)
        } else {
            rate.rate
        };
        hourly_grouped
            .entry((rate.symbol, rate.ts_hour))
            .or_default()
            .insert(rate.platform, hourly);
    }

    let seven_day_spread_apr =
        calculate_fixed_direction_spreads(&hourly_grouped, SEVEN_DAY_WINDOW_HOURS);

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{Duration, NaiveDate, NaiveDateTime};

    use super::calculate_fixed_direction_spreads;

    type HourlyFunding = HashMap<(String, NaiveDateTime), HashMap<String, f64>>;

    fn hour(offset: i64) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            + Duration::hours(offset)
    }

    fn insert_hour(rates: &mut HourlyFunding, offset: i64, platform_rates: &[(&str, f64)]) {
        rates.insert(
            ("SOL".to_string(), hour(offset)),
            platform_rates
                .iter()
                .map(|(platform, rate)| ((*platform).to_string(), *rate))
                .collect(),
        );
    }

    fn total_for(
        spreads: &HashMap<String, Vec<crate::PairSpread>>,
        long: &str,
        short: &str,
    ) -> f64 {
        spreads["SOL"]
            .iter()
            .find(|pair| pair.long_platform == long && pair.short_platform == short)
            .unwrap()
            .total_spread
    }

    fn reversal_window() -> HourlyFunding {
        let mut rates = HourlyFunding::new();
        insert_hour(&mut rates, 0, &[("hyperliquid", 0.0), ("phoenix", 0.0003)]);
        insert_hour(&mut rates, 1, &[("hyperliquid", 0.0003), ("phoenix", 0.0)]);
        insert_hour(&mut rates, 2, &[("hyperliquid", 0.0), ("phoenix", 0.0003)]);
        for offset in 3..168 {
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0), ("phoenix", 0.0)],
            );
        }
        rates
    }

    #[test]
    fn fixed_direction_includes_negative_hours_without_reversing() {
        let rates = reversal_window();

        let spreads = calculate_fixed_direction_spreads(&rates, 168);

        // The fixed long-HL/short-PHX position earns +3, loses -3, then earns
        // +3 basis points. Favorable-only accumulation would incorrectly emit +6.
        assert!((total_for(&spreads, "hyperliquid", "phoenix") - 0.03).abs() < 1e-12);
    }

    #[test]
    fn opposite_direction_is_evaluated_separately_with_the_inverse_sign() {
        let rates = reversal_window();

        let spreads = calculate_fixed_direction_spreads(&rates, 168);

        assert!((total_for(&spreads, "phoenix", "hyperliquid") + 0.03).abs() < 1e-12);
    }

    #[test]
    fn exact_168_hour_coverage_is_accepted() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0001), ("phoenix", 0.0002)],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, 168);

        assert_eq!(spreads["SOL"].len(), 2);
        assert!((total_for(&spreads, "hyperliquid", "phoenix") - 1.68).abs() < 1e-12);
    }

    #[test]
    fn missing_hour_is_excluded_instead_of_forward_filled() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            if offset == 71 {
                continue;
            }
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0001), ("phoenix", 0.0002)],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, 168);

        assert!(!spreads.contains_key("SOL"));
    }

    #[test]
    fn ranking_uses_short_minus_long_fixed_total() {
        let mut rates = HourlyFunding::new();
        insert_hour(
            &mut rates,
            0,
            &[
                ("hyperliquid", 0.0001),
                ("pacifica", 0.0004),
                ("phoenix", -0.0002),
            ],
        );

        let spreads = calculate_fixed_direction_spreads(&rates, 1);
        let best = &spreads["SOL"][0];

        assert_eq!(best.long_platform, "phoenix");
        assert_eq!(best.short_platform, "pacifica");
        assert!((best.total_spread - 0.06).abs() < 1e-12);
    }
}
