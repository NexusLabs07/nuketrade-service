use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use db::funding::{get_7d_funding_stats, get_7d_hourly_rates};
use sqlx::PgPool;
use tokio::sync::watch;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{PairSpread, SevenDayApr};

const SEVEN_DAY_WINDOW_HOURS: usize = 168;
const MIN_WINDOW_OBSERVATIONS: usize = 160;
const MAX_CONSECUTIVE_MISSING_HOURS: usize = 3;
const BEST_PAIR_HISTORY_POLICY_VERSION: &str = "best-pair-history-policy-v1";

#[derive(Default)]
struct FixedPairWindow {
    hourly_spreads: BTreeMap<chrono::NaiveDateTime, f64>,
}

struct RankingCohort {
    platforms: BTreeSet<String>,
    observed_hours: BTreeSet<chrono::NaiveDateTime>,
}

fn minimum_observed_hours(required_hours: usize) -> usize {
    debug_assert_eq!(required_hours, SEVEN_DAY_WINDOW_HOURS);
    MIN_WINDOW_OBSERVATIONS
}

fn longest_consecutive_missing_hours(
    observed_hours: &BTreeSet<chrono::NaiveDateTime>,
    window_end_hour: chrono::NaiveDateTime,
    required_hours: usize,
) -> usize {
    let window_start = window_end_hour - chrono::Duration::hours(required_hours as i64);
    let mut consecutive_missing = 0;
    let mut longest_gap = 0;
    for offset in 0..required_hours {
        let hour = window_start + chrono::Duration::hours(offset as i64);
        if observed_hours.contains(&hour) {
            consecutive_missing = 0;
        } else {
            consecutive_missing += 1;
            longest_gap = longest_gap.max(consecutive_missing);
        }
    }

    longest_gap
}

fn coverage_is_eligible(
    observed_hours: &BTreeSet<chrono::NaiveDateTime>,
    window_end_hour: chrono::NaiveDateTime,
    required_hours: usize,
) -> bool {
    if observed_hours.len() < minimum_observed_hours(required_hours) {
        return false;
    }

    longest_consecutive_missing_hours(observed_hours, window_end_hour, required_hours)
        <= MAX_CONSECUTIVE_MISSING_HOURS
}

fn projected_window_total(
    window: &FixedPairWindow,
    hours: &BTreeSet<chrono::NaiveDateTime>,
    required_hours: usize,
) -> f64 {
    let observed_total: f64 = hours
        .iter()
        .filter_map(|hour| window.hourly_spreads.get(hour))
        .sum();

    observed_total / hours.len() as f64 * required_hours as f64
}

fn select_ranking_cohort(
    venue_hours: &[(String, BTreeSet<chrono::NaiveDateTime>)],
    window_end_hour: chrono::NaiveDateTime,
    required_hours: usize,
) -> Option<RankingCohort> {
    let (_first_platform, first_hours) = venue_hours.first()?;
    let full_intersection = venue_hours
        .iter()
        .skip(1)
        .fold(first_hours.clone(), |intersection, (_platform, hours)| {
            intersection.intersection(hours).copied().collect()
        });
    if venue_hours.len() >= 2
        && coverage_is_eligible(&full_intersection, window_end_hour, required_hours)
    {
        return Some(RankingCohort {
            platforms: venue_hours
                .iter()
                .map(|(platform, _hours)| platform.clone())
                .collect(),
            observed_hours: full_intersection,
        });
    }

    let mut best_pair: Option<RankingCohort> = None;
    for i in 0..venue_hours.len() {
        for j in (i + 1)..venue_hours.len() {
            let (platform_a, hours_a) = &venue_hours[i];
            let (platform_b, hours_b) = &venue_hours[j];
            let intersection: BTreeSet<_> = hours_a.intersection(hours_b).copied().collect();
            if !coverage_is_eligible(&intersection, window_end_hour, required_hours) {
                continue;
            }

            let candidate = RankingCohort {
                platforms: [platform_a.clone(), platform_b.clone()]
                    .into_iter()
                    .collect(),
                observed_hours: intersection,
            };
            let should_replace = best_pair.as_ref().is_none_or(|current| {
                candidate.observed_hours.len() > current.observed_hours.len()
                    || (candidate.observed_hours.len() == current.observed_hours.len()
                        && candidate.platforms < current.platforms)
            });
            if should_replace {
                best_pair = Some(candidate);
            }
        }
    }

    best_pair
}

/// Computes every directed long/short pair over one unchanged position direction.
///
/// Input rates must already be normalized to decimal funding per hour and grouped
/// by symbol and UTC hour. A directed pair is emitted only when both venues have
/// at least 95% synchronized coverage and no gap longer than three hours. Missing
/// observations are never filled or treated as zero. When every venue has enough
/// shared coverage, all pairs use the same timestamps. Otherwise, only the
/// healthiest qualifying two-venue cohort is ranked, so different timestamp sets
/// never compete while one venue outage cannot blank an unaffected pair.
fn calculate_fixed_direction_spreads(
    hourly_grouped: &HashMap<(String, chrono::NaiveDateTime), HashMap<String, f64>>,
    window_end_hour: chrono::NaiveDateTime,
    required_hours: usize,
) -> HashMap<String, Vec<PairSpread>> {
    let mut windows: HashMap<(String, String, String), FixedPairWindow> = HashMap::new();
    let mut platform_hours: HashMap<(String, String), BTreeSet<chrono::NaiveDateTime>> =
        HashMap::new();

    for ((symbol, ts_hour), platforms) in hourly_grouped {
        for platform in platforms.keys() {
            platform_hours
                .entry((symbol.clone(), platform.clone()))
                .or_default()
                .insert(*ts_hour);
        }

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
                    window.hourly_spreads.insert(*ts_hour, spread);
                }
            }
        }
    }

    let mut platforms_by_symbol: HashMap<String, Vec<(String, BTreeSet<chrono::NaiveDateTime>)>> =
        HashMap::new();
    for ((symbol, platform), hours) in &platform_hours {
        platforms_by_symbol
            .entry(symbol.clone())
            .or_default()
            .push((platform.clone(), hours.clone()));
    }
    let mut cohorts_by_symbol = HashMap::new();
    for (symbol, mut venue_hours) in platforms_by_symbol {
        venue_hours.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some(cohort) = select_ranking_cohort(&venue_hours, window_end_hour, required_hours) {
            let platforms = cohort
                .platforms
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",");
            let longest_gap = longest_consecutive_missing_hours(
                &cohort.observed_hours,
                window_end_hour,
                required_hours,
            );
            log::info!(
                "best_pair_history_policy policy_id={} symbol={} cohort={} observed_hours={} expected_hours={} longest_gap_hours={}",
                BEST_PAIR_HISTORY_POLICY_VERSION,
                symbol,
                platforms,
                cohort.observed_hours.len(),
                required_hours,
                longest_gap,
            );
            cohorts_by_symbol.insert(symbol, cohort);
        }
    }

    let mut spreads: HashMap<String, Vec<PairSpread>> = HashMap::new();
    for ((symbol, long_platform, short_platform), window) in windows {
        let Some(cohort) = cohorts_by_symbol.get(&symbol) else {
            continue;
        };
        if !cohort.platforms.contains(&long_platform) || !cohort.platforms.contains(&short_platform)
        {
            continue;
        }

        spreads.entry(symbol).or_default().push(PairSpread {
            long_platform,
            short_platform,
            total_spread: projected_window_total(&window, &cohort.observed_hours, required_hours),
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
    let window_end_hour = hourly_rates.first().map(|rate| rate.window_end_hour);

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

    let seven_day_spread_apr = window_end_hour
        .map(|end_hour| {
            calculate_fixed_direction_spreads(&hourly_grouped, end_hour, SEVEN_DAY_WINDOW_HOURS)
        })
        .unwrap_or_default();

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

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        // The fixed long-HL/short-PHX position earns +3, loses -3, then earns
        // +3 basis points. Favorable-only accumulation would incorrectly emit +6.
        assert!((total_for(&spreads, "hyperliquid", "phoenix") - 0.03).abs() < 1e-12);
    }

    #[test]
    fn opposite_direction_is_evaluated_separately_with_the_inverse_sign() {
        let rates = reversal_window();

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

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

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        assert_eq!(spreads["SOL"].len(), 2);
        assert!((total_for(&spreads, "hyperliquid", "phoenix") - 1.68).abs() < 1e-12);
    }

    #[test]
    fn consecutive_gap_above_limit_is_excluded_instead_of_forward_filled() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            if (70..74).contains(&offset) {
                continue;
            }
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0001), ("phoenix", 0.0002)],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        assert!(!spreads.contains_key("SOL"));
    }

    #[test]
    fn short_outage_keeps_pair_available_without_treating_missing_hours_as_zero() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            if (70..73).contains(&offset) {
                continue;
            }
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0001), ("phoenix", 0.0002)],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        // The observed hourly mean is projected over the complete seven-day
        // window. Missing observations are neither forward-filled nor counted
        // as zero-return hours.
        assert!((total_for(&spreads, "hyperliquid", "phoenix") - 1.68).abs() < 1e-12);
    }

    #[test]
    fn coverage_below_95_percent_is_excluded() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            if offset % 20 == 0 {
                continue;
            }
            insert_hour(
                &mut rates,
                offset,
                &[("hyperliquid", 0.0001), ("phoenix", 0.0002)],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        assert!(!spreads.contains_key("SOL"));
    }

    #[test]
    fn healthy_three_venue_pairs_use_the_same_common_hours() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            let mut platform_rates = vec![("phoenix", -0.0001)];
            if offset != 10 {
                platform_rates.push(("hyperliquid", 0.0001));
            }
            if offset != 30 {
                // This outlier would distort PAC/PHX if that pair used its own
                // 167-hour set instead of the shared three-venue timestamps.
                let pacifica_rate = if offset == 10 { 0.01 } else { 0.0002 };
                platform_rates.push(("pacifica", pacifica_rate));
            }
            insert_hour(&mut rates, offset, &platform_rates);
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        assert_eq!(spreads["SOL"].len(), 6);
        assert!((total_for(&spreads, "phoenix", "pacifica") - 5.04).abs() < 1e-12);
    }

    #[test]
    fn major_single_venue_outage_preserves_healthy_remaining_pair() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            let mut platform_rates = vec![("pacifica", 0.0002), ("phoenix", -0.0001)];
            if !(50..80).contains(&offset) {
                platform_rates.push(("hyperliquid", 0.0001));
            }
            insert_hour(&mut rates, offset, &platform_rates);
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        assert_eq!(spreads["SOL"].len(), 2);
        assert!(spreads["SOL"].iter().all(|pair| {
            pair.long_platform != "hyperliquid" && pair.short_platform != "hyperliquid"
        }));
    }

    #[test]
    fn failed_three_venue_intersection_selects_one_consistent_pair_cohort() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            let mut platform_rates = vec![("phoenix", -0.0001)];
            if ![0, 20, 40, 60, 80, 100, 120, 140].contains(&offset) {
                platform_rates.push(("hyperliquid", 0.0001));
            }
            if ![10, 30, 50].contains(&offset) {
                platform_rates.push(("pacifica", 0.0002));
            }
            insert_hour(&mut rates, offset, &platform_rates);
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);

        // The three-way intersection has only 157 hours. PAC/PHX is the
        // healthiest qualifying two-venue cohort at 165 hours, so HL/PHX must
        // not compete using a different 160-hour timestamp set.
        assert_eq!(spreads["SOL"].len(), 2);
        assert!(spreads["SOL"].iter().all(|pair| {
            pair.long_platform != "hyperliquid" && pair.short_platform != "hyperliquid"
        }));
    }

    #[test]
    fn ranking_uses_short_minus_long_fixed_total() {
        let mut rates = HourlyFunding::new();
        for offset in 0..168 {
            insert_hour(
                &mut rates,
                offset,
                &[
                    ("hyperliquid", 0.0001),
                    ("pacifica", 0.0004),
                    ("phoenix", -0.0002),
                ],
            );
        }

        let spreads = calculate_fixed_direction_spreads(&rates, hour(168), 168);
        let best = &spreads["SOL"][0];

        assert_eq!(best.long_platform, "phoenix");
        assert_eq!(best.short_platform, "pacifica");
        assert!((best.total_spread - 10.08).abs() < 1e-12);
    }
}
