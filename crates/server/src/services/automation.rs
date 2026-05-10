//! Automation recommendation: pure best-pair selection.
//!
//! All thresholds (`min_apr_to_enter`, `exit_if_apr_below`, etc.) are
//! interpreted in **annualized APR %**. NET mode uses a single live-feed
//! snapshot; SEVEN_D mode reads the cron-computed 7d cumulative spread and
//! extrapolates to APR. See the plan in `.cursor/plans/` for the locked unit
//! contract.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use perp_core::SevenDayApr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::FeedSnapshot;

// ============================= Constants =============================

pub mod metric_mode {
    pub const NET: &str = "NET";
    pub const SEVEN_D: &str = "SEVEN_D";
}

pub mod recommended_action {
    pub const OPEN: &str = "OPEN";
    pub const HOLD: &str = "HOLD";
    pub const CLOSE: &str = "CLOSE";
    pub const REBALANCE: &str = "REBALANCE";
    /// Forced close path — used for runs in STOPPING that still hold a
    /// position. Bypasses cooldown / hysteresis.
    pub const EMERGENCY_CLOSE: &str = "EMERGENCY_CLOSE";
    pub const NOOP_BELOW_MIN: &str = "NOOP_BELOW_MIN";
}

/// Failure-policy constants used by the result-callback path.
pub const MAX_CONSECUTIVE_FAILURES_BEFORE_RUN_FAILED: i32 = 3;
/// Default lease TTL for leased intents handed to the Node executor.
pub const DEFAULT_LEASE_TTL_SEC: i64 = 60;
/// Cadence at which `automation_runs` are re-evaluated by the Node poller.
pub const EVAL_INTERVAL_SEC: i64 = 15;

pub mod side {
    pub const LONG: &str = "LONG";
    pub const SHORT: &str = "SHORT";
}

/// Hours per year — annualization factor for hourly-funding venues.
/// HL and Pacifica both fund hourly. If a non-hourly venue is added, promote
/// this to a per-exchange constant.
pub const HOURS_PER_YEAR: f64 = 24.0 * 365.0;

/// Default poll interval used to bucket `as_of_bucket` so identical decisions
/// within the same window dedupe via the unique constraint on `automation_actions`.
pub const DEFAULT_POLL_INTERVAL_SEC: i64 = 15;

// ============================= Types =============================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendedLeg {
    pub exchange: String,
    /// "LONG" | "SHORT"
    pub side: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegFundingInput {
    pub exchange: String,
    pub funding: f64,
    pub max_leverage: Option<u32>,
}

/// Reference price snapshot used by the Node executor for slippage limits
/// and order sizing. Stringified to preserve precision over JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencePrice {
    pub symbol: String,
    /// Decimal string. v1 uses the long-leg's mark price (with short-leg as
    /// fallback) — see `resolve_reference_price`.
    pub px: String,
    /// Source of the price field. v1 only has `mark` — when more sources
    /// (mid/index) are wired up, expand the values here.
    pub source: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub decision_id: Uuid,
    pub decision_hash: String,
    pub asset: Option<String>,
    pub legs: Vec<RecommendedLeg>,
    pub metric_mode: String,
    pub metric_value_apr_pct: f64,
    pub metric_value_raw: f64,
    pub inputs: Vec<LegFundingInput>,
    pub as_of_ts: i64,
    pub as_of_bucket: i64,
    pub recommended_action: String,
    pub reasons: Vec<String>,
    /// Present when an actionable recommendation has a known asset and the
    /// live feed has a usable mark price for either leg.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_price: Option<ReferencePrice>,
}

impl Recommendation {
    /// A "no opportunity" output (e.g. no eligible candidates).
    pub fn empty(metric_mode: &str, as_of_ts: i64, as_of_bucket: i64, reasons: Vec<String>) -> Self {
        Self {
            decision_id: Uuid::new_v4(),
            decision_hash: compute_decision_hash(
                None,
                &Vec::<RecommendedLeg>::new(),
                metric_mode,
                as_of_bucket,
            ),
            asset: None,
            legs: vec![],
            metric_mode: metric_mode.to_string(),
            metric_value_apr_pct: 0.0,
            metric_value_raw: 0.0,
            inputs: vec![],
            as_of_ts,
            as_of_bucket,
            recommended_action: recommended_action::NOOP_BELOW_MIN.to_string(),
            reasons,
            reference_price: None,
        }
    }
}

/// Effective config used by the recommender. Comes either from the current
/// `automation_configs` row (preview) or from the run's snapshot fields
/// (live executor evaluation).
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub apr_mode: String,
    pub min_apr_to_enter: f64,
    pub exit_if_apr_below: f64,
    pub rebalance_to_better_pair: bool,
    pub min_rebalance_improvement_bps: i32,
    pub min_time_between_actions_sec: i32,
    pub cooldown_after_error_sec: i32,
    pub max_leverage: f64,
    pub max_actions_per_day: i32,
    pub excluded_assets: Vec<String>,
    pub allowed_exchanges: Vec<String>,
    /// Pass-through to the intent payload; not used in selection logic.
    pub max_slippage_bps: i32,
    /// Pass-through to the intent payload; not used in selection logic.
    pub reduce_only_on_close: bool,
}

/// Optional current run state. When None the recommender treats this as a
/// "preview" and only suggests OPEN / NOOP_BELOW_MIN.
#[derive(Debug, Clone, Default)]
pub struct CurrentRunState {
    pub current_asset: Option<String>,
    pub current_legs: Vec<RecommendedLeg>,
    pub last_action_at: Option<DateTime<Utc>>,
    pub actions_today: i32,
}

// ============================= Pure entry point =============================

/// Compute the best HL↔Pacifica (or other allowed pair) recommendation.
pub fn compute_best_pair(
    feed: &FeedSnapshot,
    seven_day: &SevenDayApr,
    config: &EffectiveConfig,
    current: Option<&CurrentRunState>,
    now: DateTime<Utc>,
    poll_interval_sec: i64,
) -> Recommendation {
    let metric_mode = if config.apr_mode == metric_mode::SEVEN_D {
        metric_mode::SEVEN_D
    } else {
        metric_mode::NET
    };
    let as_of_ts = now.timestamp();
    let as_of_bucket = if poll_interval_sec > 0 {
        as_of_ts / poll_interval_sec
    } else {
        as_of_ts
    };

    let mut reasons: Vec<String> = vec![];

    let allowed: Vec<String> = config
        .allowed_exchanges
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    if allowed.len() < 2 {
        reasons.push("not_enough_allowed_exchanges".into());
        return Recommendation::empty(metric_mode, as_of_ts, as_of_bucket, reasons);
    }
    let exclude_set: std::collections::HashSet<String> = config
        .excluded_assets
        .iter()
        .map(|s| s.to_uppercase())
        .collect();

    // Build candidate list per metric mode.
    let candidates = match metric_mode {
        metric_mode::NET => candidates_from_live(feed, &allowed, &exclude_set, config.max_leverage),
        metric_mode::SEVEN_D => candidates_from_seven_d(seven_day, &allowed, &exclude_set),
        _ => vec![],
    };

    if candidates.is_empty() {
        reasons.push(if metric_mode == metric_mode::NET {
            "no_live_candidates".into()
        } else {
            "no_seven_d_candidates".into()
        });
        return Recommendation::empty(metric_mode, as_of_ts, as_of_bucket, reasons);
    }

    let best = candidates
        .into_iter()
        .max_by(|a, b| {
            a.metric_value_apr_pct
                .partial_cmp(&b.metric_value_apr_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("non-empty after check");

    let recommended_action = decide_action(&best, config, current, now, &mut reasons);

    let legs = vec![
        RecommendedLeg {
            exchange: best.long_exchange.clone(),
            side: side::LONG.to_string(),
            weight: 1.0,
        },
        RecommendedLeg {
            exchange: best.short_exchange.clone(),
            side: side::SHORT.to_string(),
            weight: 1.0,
        },
    ];
    let decision_hash = compute_decision_hash(Some(&best.asset), &legs, metric_mode, as_of_bucket);
    let reference_price =
        resolve_reference_price(feed, &best.asset, &best.long_exchange, &best.short_exchange, now);

    Recommendation {
        decision_id: Uuid::new_v4(),
        decision_hash,
        asset: Some(best.asset.clone()),
        legs,
        metric_mode: metric_mode.to_string(),
        metric_value_apr_pct: best.metric_value_apr_pct,
        metric_value_raw: best.metric_value_raw,
        inputs: best.inputs,
        as_of_ts,
        as_of_bucket,
        recommended_action,
        reasons,
        reference_price,
    }
}

/// Build an EMERGENCY_CLOSE recommendation for a run that is STOPPING and
/// still has a current position. Bypasses cooldown, hysteresis, and
/// best-pair selection — it forces a close on the run's current pair.
pub fn compute_emergency_close(
    feed: &FeedSnapshot,
    apr_mode: &str,
    asset: &str,
    current_legs: &[RecommendedLeg],
    now: DateTime<Utc>,
    poll_interval_sec: i64,
) -> Recommendation {
    let metric_mode = if apr_mode == metric_mode::SEVEN_D {
        metric_mode::SEVEN_D
    } else {
        metric_mode::NET
    };
    let as_of_ts = now.timestamp();
    let as_of_bucket = if poll_interval_sec > 0 {
        as_of_ts / poll_interval_sec
    } else {
        as_of_ts
    };
    let legs: Vec<RecommendedLeg> = current_legs.to_vec();
    let decision_hash = compute_decision_hash(Some(asset), &legs, metric_mode, as_of_bucket);
    let long_exchange = legs
        .iter()
        .find(|l| l.side == side::LONG)
        .map(|l| l.exchange.clone())
        .unwrap_or_default();
    let short_exchange = legs
        .iter()
        .find(|l| l.side == side::SHORT)
        .map(|l| l.exchange.clone())
        .unwrap_or_default();
    let reference_price = resolve_reference_price(feed, asset, &long_exchange, &short_exchange, now);

    Recommendation {
        decision_id: Uuid::new_v4(),
        decision_hash,
        asset: Some(asset.to_string()),
        legs,
        metric_mode: metric_mode.to_string(),
        metric_value_apr_pct: 0.0,
        metric_value_raw: 0.0,
        inputs: vec![],
        as_of_ts,
        as_of_bucket,
        recommended_action: recommended_action::EMERGENCY_CLOSE.to_string(),
        reasons: vec!["run_stopping_with_position".into()],
        reference_price,
    }
}

/// Pick a mark-price reference for an asset. v1 prefers the long-leg's
/// `mark_px`, falling back to the short-leg's `mark_px`. Returns `None` if
/// no live row or no mark price is available.
fn resolve_reference_price(
    feed: &FeedSnapshot,
    asset: &str,
    long_exchange: &str,
    short_exchange: &str,
    now: DateTime<Utc>,
) -> Option<ReferencePrice> {
    let row = feed.by_symbol.get(asset)?;
    let pick = |ex: &str| -> Option<f64> {
        match ex.to_lowercase().as_str() {
            "hyperliquid" => row.hyperliquid.as_ref().and_then(|v| v.mark_px),
            "pacifica" => row.pacifica.as_ref().and_then(|v| v.mark_px),
            "backpack" => row.backpack.as_ref().and_then(|v| v.mark_px),
            "lighter" => row.lighter.as_ref().and_then(|v| v.mark_px),
            _ => None,
        }
    };
    let px = pick(long_exchange).or_else(|| pick(short_exchange))?;
    Some(ReferencePrice {
        symbol: asset.to_string(),
        px: format_decimal(px),
        source: "mark".to_string(),
        ts_ms: now.timestamp_millis(),
    })
}

/// Format an f64 as a decimal string without scientific notation, trimming
/// trailing zeros after the decimal point.
fn format_decimal(v: f64) -> String {
    let s = format!("{v:.10}");
    if !s.contains('.') {
        return s;
    }
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

// ============================= Internals =============================

#[derive(Debug, Clone)]
struct Candidate {
    asset: String,
    long_exchange: String,
    short_exchange: String,
    metric_value_apr_pct: f64,
    metric_value_raw: f64,
    inputs: Vec<LegFundingInput>,
}

fn candidates_from_live(
    feed: &FeedSnapshot,
    allowed: &[String],
    exclude_set: &std::collections::HashSet<String>,
    max_leverage_cap: f64,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (symbol, row) in feed.by_symbol.iter() {
        if exclude_set.contains(&symbol.to_uppercase()) {
            continue;
        }
        // Collect (exchange_id_lowercase, funding, max_leverage)
        let mut legs: Vec<(String, f64, Option<u32>)> = Vec::new();
        if allowed.iter().any(|e| e == "hyperliquid") {
            if let Some(v) = row.hyperliquid.as_ref() {
                if let Some(funding) = v.funding {
                    legs.push(("hyperliquid".to_string(), funding, v.max_leverage));
                }
            }
        }
        if allowed.iter().any(|e| e == "pacifica") {
            if let Some(v) = row.pacifica.as_ref() {
                if let Some(funding) = v.funding {
                    legs.push(("pacifica".to_string(), funding, v.max_leverage));
                }
            }
        }
        if allowed.iter().any(|e| e == "backpack") {
            if let Some(v) = row.backpack.as_ref() {
                if let Some(funding) = v.funding {
                    legs.push(("backpack".to_string(), funding, v.max_leverage));
                }
            }
        }
        if allowed.iter().any(|e| e == "lighter") {
            if let Some(v) = row.lighter.as_ref() {
                if let Some(funding) = v.funding {
                    legs.push(("lighter".to_string(), funding, v.max_leverage));
                }
            }
        }
        if legs.len() < 2 {
            continue;
        }
        // Filter on max_leverage availability for both candidate legs we use.
        if max_leverage_cap > 1.0 {
            let needed = max_leverage_cap as u32;
            // Keep legs whose max_leverage is unknown OR >= cap.
            legs.retain(|(_, _, ml)| ml.map(|m| m >= needed).unwrap_or(true));
            if legs.len() < 2 {
                continue;
            }
        }

        // Pairwise: pick the pair with the largest |spread|.
        let mut best: Option<(String, String, f64, f64, Vec<LegFundingInput>)> = None;
        for i in 0..legs.len() {
            for j in (i + 1)..legs.len() {
                let (ref a_name, a_rate, a_lev) = legs[i];
                let (ref b_name, b_rate, b_lev) = legs[j];
                if (a_rate - b_rate).abs() <= f64::EPSILON {
                    continue;
                }
                let (long_name, long_rate, long_lev, short_name, short_rate, short_lev) =
                    if a_rate < b_rate {
                        (a_name, a_rate, a_lev, b_name, b_rate, b_lev)
                    } else {
                        (b_name, b_rate, b_lev, a_name, a_rate, a_lev)
                    };
                let raw_per_hour = short_rate - long_rate;
                let apr_pct = raw_per_hour * HOURS_PER_YEAR * 100.0;
                let inputs = vec![
                    LegFundingInput {
                        exchange: long_name.clone(),
                        funding: long_rate,
                        max_leverage: long_lev,
                    },
                    LegFundingInput {
                        exchange: short_name.clone(),
                        funding: short_rate,
                        max_leverage: short_lev,
                    },
                ];
                if best
                    .as_ref()
                    .map(|b| apr_pct > b.3)
                    .unwrap_or(true)
                {
                    best = Some((
                        long_name.clone(),
                        short_name.clone(),
                        raw_per_hour,
                        apr_pct,
                        inputs,
                    ));
                }
            }
        }
        if let Some((long_exchange, short_exchange, raw, apr_pct, inputs)) = best {
            out.push(Candidate {
                asset: symbol.clone(),
                long_exchange,
                short_exchange,
                metric_value_apr_pct: apr_pct,
                metric_value_raw: raw,
                inputs,
            });
        }
    }
    out
}

fn candidates_from_seven_d(
    seven_day: &SevenDayApr,
    allowed: &[String],
    exclude_set: &std::collections::HashSet<String>,
) -> Vec<Candidate> {
    // Annualize: total_spread is cumulative percent over 7d → APR % = ts * (365/7).
    let factor = 365.0 / 7.0;
    let mut out = Vec::new();
    for (symbol, pairs) in seven_day.seven_day_spread_apr.iter() {
        if exclude_set.contains(&symbol.to_uppercase()) {
            continue;
        }
        let mut best: Option<&perp_core::PairSpread> = None;
        for pair in pairs {
            let l = pair.long_platform.to_lowercase();
            let s = pair.short_platform.to_lowercase();
            if !allowed.iter().any(|e| e == &l) {
                continue;
            }
            if !allowed.iter().any(|e| e == &s) {
                continue;
            }
            if best
                .map(|b| pair.total_spread > b.total_spread)
                .unwrap_or(true)
            {
                best = Some(pair);
            }
        }
        if let Some(b) = best {
            let apr_pct = b.total_spread * factor;
            let avg = seven_day.seven_day_avg_apr.get(symbol);
            let long_rate = avg
                .and_then(|m| m.get(&b.long_platform).copied())
                .unwrap_or(0.0);
            let short_rate = avg
                .and_then(|m| m.get(&b.short_platform).copied())
                .unwrap_or(0.0);
            let inputs = vec![
                LegFundingInput {
                    exchange: b.long_platform.clone(),
                    funding: long_rate,
                    max_leverage: None,
                },
                LegFundingInput {
                    exchange: b.short_platform.clone(),
                    funding: short_rate,
                    max_leverage: None,
                },
            ];
            out.push(Candidate {
                asset: symbol.clone(),
                long_exchange: b.long_platform.clone(),
                short_exchange: b.short_platform.clone(),
                metric_value_apr_pct: apr_pct,
                metric_value_raw: b.total_spread,
                inputs,
            });
        }
    }
    out
}

fn decide_action(
    best: &Candidate,
    config: &EffectiveConfig,
    current: Option<&CurrentRunState>,
    now: DateTime<Utc>,
    reasons: &mut Vec<String>,
) -> String {
    let has_position = current
        .map(|s| s.current_asset.is_some() && !s.current_legs.is_empty())
        .unwrap_or(false);

    // Daily cap (only enforces NEW actions).
    if let Some(state) = current {
        if config.max_actions_per_day > 0 && state.actions_today >= config.max_actions_per_day {
            reasons.push("max_actions_per_day_reached".into());
            return if has_position {
                recommended_action::HOLD.to_string()
            } else {
                recommended_action::NOOP_BELOW_MIN.to_string()
            };
        }
    }

    // No-position branch.
    if !has_position {
        return if best.metric_value_apr_pct >= config.min_apr_to_enter {
            recommended_action::OPEN.to_string()
        } else {
            reasons.push("below_min_apr_to_enter".into());
            recommended_action::NOOP_BELOW_MIN.to_string()
        };
    }

    // Has-position branch.
    let state = current.expect("checked above");
    let same_pair = state
        .current_asset
        .as_deref()
        .map(|a| a == best.asset)
        .unwrap_or(false)
        && pair_matches(&state.current_legs, &best.long_exchange, &best.short_exchange);

    let cooldown_elapsed = state
        .last_action_at
        .map(|t| (now - t).num_seconds() >= config.min_time_between_actions_sec as i64)
        .unwrap_or(true);

    // Exit if metric below exit threshold for the current pair.
    // Note: the recommender's `best` is the global best; if it's a different
    // pair, we still rely on metric_value_apr_pct of `best` as a proxy floor —
    // but the real exit decision should compare current pair's metric. The
    // executor refines this with explicit current-pair metric lookup; here we
    // emit CLOSE only when even the best is below exit threshold.
    if best.metric_value_apr_pct < config.exit_if_apr_below {
        reasons.push("best_below_exit_threshold".into());
        if cooldown_elapsed {
            return recommended_action::CLOSE.to_string();
        }
        reasons.push("cooldown_active".into());
        return recommended_action::HOLD.to_string();
    }

    if same_pair {
        reasons.push("same_pair".into());
        return recommended_action::HOLD.to_string();
    }

    if !config.rebalance_to_better_pair {
        reasons.push("rebalance_disabled".into());
        return recommended_action::HOLD.to_string();
    }

    // Rebalance only when improvement clears hysteresis AND cooldown elapsed.
    let improvement_threshold_apr_pct = config.min_rebalance_improvement_bps as f64 / 100.0;
    // The current pair's APR % is unknown to the pure recommender without a
    // lookup — the executor passes the precomputed current_pair_apr separately
    // when calling. As a conservative default, require the BEST be at least
    // (min_apr_to_enter + improvement_threshold) above zero so we don't churn
    // on noise even if improvement can't be measured precisely here.
    if best.metric_value_apr_pct < (config.min_apr_to_enter + improvement_threshold_apr_pct) {
        reasons.push("improvement_below_hysteresis".into());
        return recommended_action::HOLD.to_string();
    }

    if !cooldown_elapsed {
        reasons.push("cooldown_active".into());
        return recommended_action::HOLD.to_string();
    }

    recommended_action::REBALANCE.to_string()
}

fn pair_matches(current: &[RecommendedLeg], long_ex: &str, short_ex: &str) -> bool {
    let mut has_long = false;
    let mut has_short = false;
    for leg in current {
        if leg.side == side::LONG && leg.exchange.eq_ignore_ascii_case(long_ex) {
            has_long = true;
        }
        if leg.side == side::SHORT && leg.exchange.eq_ignore_ascii_case(short_ex) {
            has_short = true;
        }
    }
    has_long && has_short
}

// ============================= Hashing helpers =============================

/// FNV-1a 64-bit hash. Deterministic across Rust toolchain versions and
/// portable; not cryptographic. Sufficient for idempotency keys.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}

/// Stable hash of a legs array, order-independent over (exchange, side).
pub fn compute_legs_hash(legs: &[RecommendedLeg]) -> String {
    let mut sorted: Vec<(String, String)> = legs
        .iter()
        .map(|l| (l.exchange.to_lowercase(), l.side.to_uppercase()))
        .collect();
    sorted.sort();
    let mut buf = String::new();
    for (e, s) in sorted {
        buf.push_str(&e);
        buf.push('|');
        buf.push_str(&s);
        buf.push(';');
    }
    fnv1a64_hex(buf.as_bytes())
}

pub fn compute_decision_hash(
    asset: Option<&str>,
    legs: &[RecommendedLeg],
    metric_mode: &str,
    as_of_bucket: i64,
) -> String {
    let mut buf = String::new();
    buf.push_str(asset.unwrap_or(""));
    buf.push('|');
    buf.push_str(&compute_legs_hash(legs));
    buf.push('|');
    buf.push_str(metric_mode);
    buf.push('|');
    buf.push_str(&as_of_bucket.to_string());
    fnv1a64_hex(buf.as_bytes())
}

/// Stable hash of an effective config, used to populate `automation_runs.config_hash`.
pub fn compute_config_hash(cfg: &EffectiveConfig) -> String {
    // BTreeMap to ensure stable ordering of vec fields' hash inputs.
    let mut excluded: Vec<String> = cfg.excluded_assets.iter().map(|s| s.to_uppercase()).collect();
    excluded.sort();
    let mut allowed: Vec<String> = cfg.allowed_exchanges.iter().map(|s| s.to_lowercase()).collect();
    allowed.sort();

    let mut buf = String::new();
    let kv: BTreeMap<&str, String> = BTreeMap::from([
        ("apr_mode", cfg.apr_mode.clone()),
        ("min_apr_to_enter", format!("{:.10}", cfg.min_apr_to_enter)),
        ("exit_if_apr_below", format!("{:.10}", cfg.exit_if_apr_below)),
        (
            "rebalance_to_better_pair",
            cfg.rebalance_to_better_pair.to_string(),
        ),
        (
            "min_rebalance_improvement_bps",
            cfg.min_rebalance_improvement_bps.to_string(),
        ),
        (
            "min_time_between_actions_sec",
            cfg.min_time_between_actions_sec.to_string(),
        ),
        (
            "cooldown_after_error_sec",
            cfg.cooldown_after_error_sec.to_string(),
        ),
        ("max_leverage", format!("{:.10}", cfg.max_leverage)),
        (
            "max_actions_per_day",
            cfg.max_actions_per_day.to_string(),
        ),
        ("excluded_assets", excluded.join(",")),
        ("allowed_exchanges", allowed.join(",")),
    ]);
    for (k, v) in kv {
        buf.push_str(k);
        buf.push('=');
        buf.push_str(&v);
        buf.push(';');
    }
    fnv1a64_hex(buf.as_bytes())
}

// ============================= Tests =============================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::types::{LiveMarketFeedResponse, MarketFeedValueStruct};

    fn make_feed(symbol: &str, hl: f64, pac: f64) -> FeedSnapshot {
        let mut by_symbol = HashMap::new();
        by_symbol.insert(
            symbol.to_string(),
            LiveMarketFeedResponse {
                symbol: symbol.to_string(),
                hyperliquid: Some(MarketFeedValueStruct {
                    mark_px: Some(50000.0),
                    funding: Some(hl),
                    max_leverage: Some(50),
                }),
                pacifica: Some(MarketFeedValueStruct {
                    mark_px: Some(50001.0),
                    funding: Some(pac),
                    max_leverage: Some(50),
                }),
                backpack: None,
                lighter: None,
            },
        );
        FeedSnapshot {
            by_symbol,
            formatted: vec![],
        }
    }

    fn default_config() -> EffectiveConfig {
        EffectiveConfig {
            apr_mode: metric_mode::NET.into(),
            min_apr_to_enter: 4.0,
            exit_if_apr_below: 2.0,
            rebalance_to_better_pair: true,
            min_rebalance_improvement_bps: 50,
            min_time_between_actions_sec: 300,
            cooldown_after_error_sec: 900,
            max_leverage: 3.0,
            max_actions_per_day: 20,
            excluded_assets: vec![],
            allowed_exchanges: vec!["hyperliquid".into(), "pacifica".into()],
            max_slippage_bps: 50,
            reduce_only_on_close: true,
        }
    }

    #[test]
    fn net_mode_picks_higher_funding_as_short() {
        // HL=0.000010 / hr, Pacifica=0.000050 / hr
        // spread/hr = 0.00004; APR = 0.00004 * 8760 * 100 = 35.04%
        let feed = make_feed("BTC", 0.000010, 0.000050);
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        };
        let cfg = default_config();
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, None, now, 15);
        assert_eq!(r.asset.as_deref(), Some("BTC"));
        assert_eq!(r.legs.len(), 2);
        let long = r.legs.iter().find(|l| l.side == side::LONG).unwrap();
        let short = r.legs.iter().find(|l| l.side == side::SHORT).unwrap();
        assert_eq!(long.exchange, "hyperliquid");
        assert_eq!(short.exchange, "pacifica");
        assert!(r.metric_value_apr_pct > 30.0);
        assert_eq!(r.recommended_action, recommended_action::OPEN);
    }

    #[test]
    fn net_mode_below_min_returns_noop() {
        // Tiny spread → APR well below 4% threshold.
        let feed = make_feed("BTC", 0.0000001, 0.0000002);
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        };
        let cfg = default_config();
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, None, now, 15);
        assert_eq!(r.recommended_action, recommended_action::NOOP_BELOW_MIN);
    }

    #[test]
    fn excluded_asset_is_skipped() {
        let feed = make_feed("BTC", 0.000010, 0.000050);
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        };
        let mut cfg = default_config();
        cfg.excluded_assets = vec!["btc".into()];
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, None, now, 15);
        assert!(r.asset.is_none());
        assert_eq!(r.recommended_action, recommended_action::NOOP_BELOW_MIN);
    }

    #[test]
    fn seven_d_mode_extrapolates_to_apr() {
        let mut spread_apr: HashMap<String, Vec<perp_core::PairSpread>> = HashMap::new();
        spread_apr.insert(
            "ETH".into(),
            vec![perp_core::PairSpread {
                long_platform: "hyperliquid".into(),
                short_platform: "pacifica".into(),
                total_spread: 1.0, // 1.0 = 1 percentage point cumulative over 7d
            }],
        );
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: spread_apr,
        };
        let feed = FeedSnapshot {
            by_symbol: HashMap::new(),
            formatted: vec![],
        };
        let mut cfg = default_config();
        cfg.apr_mode = metric_mode::SEVEN_D.into();
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, None, now, 15);
        assert_eq!(r.asset.as_deref(), Some("ETH"));
        // 1.0 * (365/7) ≈ 52.14
        assert!((r.metric_value_apr_pct - 52.142857).abs() < 0.1);
        assert_eq!(r.recommended_action, recommended_action::OPEN);
    }

    #[test]
    fn legs_hash_is_order_independent_within_same_pair() {
        let a = vec![
            RecommendedLeg {
                exchange: "hyperliquid".into(),
                side: "LONG".into(),
                weight: 1.0,
            },
            RecommendedLeg {
                exchange: "pacifica".into(),
                side: "SHORT".into(),
                weight: 1.0,
            },
        ];
        let b = vec![
            RecommendedLeg {
                exchange: "pacifica".into(),
                side: "SHORT".into(),
                weight: 1.0,
            },
            RecommendedLeg {
                exchange: "hyperliquid".into(),
                side: "LONG".into(),
                weight: 1.0,
            },
        ];
        assert_eq!(compute_legs_hash(&a), compute_legs_hash(&b));
    }

    #[test]
    fn legs_hash_differs_for_different_pair() {
        let a = vec![
            RecommendedLeg {
                exchange: "hyperliquid".into(),
                side: "LONG".into(),
                weight: 1.0,
            },
            RecommendedLeg {
                exchange: "pacifica".into(),
                side: "SHORT".into(),
                weight: 1.0,
            },
        ];
        // Sides flipped — different decision.
        let b = vec![
            RecommendedLeg {
                exchange: "pacifica".into(),
                side: "LONG".into(),
                weight: 1.0,
            },
            RecommendedLeg {
                exchange: "hyperliquid".into(),
                side: "SHORT".into(),
                weight: 1.0,
            },
        ];
        assert_ne!(compute_legs_hash(&a), compute_legs_hash(&b));
    }

    #[test]
    fn has_position_same_pair_returns_hold() {
        let feed = make_feed("BTC", 0.000010, 0.000050);
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        };
        let cfg = default_config();
        let current = CurrentRunState {
            current_asset: Some("BTC".into()),
            current_legs: vec![
                RecommendedLeg {
                    exchange: "hyperliquid".into(),
                    side: "LONG".into(),
                    weight: 1.0,
                },
                RecommendedLeg {
                    exchange: "pacifica".into(),
                    side: "SHORT".into(),
                    weight: 1.0,
                },
            ],
            last_action_at: None,
            actions_today: 0,
        };
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, Some(&current), now, 15);
        assert_eq!(r.recommended_action, recommended_action::HOLD);
    }

    #[test]
    fn config_hash_stable_for_same_inputs() {
        let cfg = default_config();
        assert_eq!(compute_config_hash(&cfg), compute_config_hash(&cfg));
    }

    #[test]
    fn emergency_close_for_stopping_run_with_position() {
        let feed = make_feed("BTC", 0.000010, 0.000050);
        let current_legs = vec![
            RecommendedLeg {
                exchange: "hyperliquid".into(),
                side: side::LONG.into(),
                weight: 1.0,
            },
            RecommendedLeg {
                exchange: "pacifica".into(),
                side: side::SHORT.into(),
                weight: 1.0,
            },
        ];
        let now = chrono::Utc::now();
        let r = compute_emergency_close(&feed, metric_mode::NET, "BTC", &current_legs, now, 15);
        assert_eq!(r.recommended_action, recommended_action::EMERGENCY_CLOSE);
        assert_eq!(r.asset.as_deref(), Some("BTC"));
        assert_eq!(r.legs.len(), 2);
        assert!(r.reasons.iter().any(|s| s == "run_stopping_with_position"));
        let rp = r.reference_price.expect("reference_price set");
        assert_eq!(rp.symbol, "BTC");
        assert_eq!(rp.source, "mark");
        // Fixture used 50_000 for HL mark_px; long leg is HL → reference_price.px = 50000
        assert_eq!(rp.px, "50000");
    }

    #[test]
    fn open_recommendation_includes_string_reference_price() {
        let feed = make_feed("BTC", 0.000010, 0.000050);
        let seven = SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        };
        let cfg = default_config();
        let now = chrono::Utc::now();
        let r = compute_best_pair(&feed, &seven, &cfg, None, now, 15);
        assert_eq!(r.recommended_action, recommended_action::OPEN);
        let rp = r.reference_price.clone().expect("reference_price set");
        assert_eq!(rp.symbol, "BTC");
        assert_eq!(rp.source, "mark");
        // long_exchange is HL so px should be HL.mark_px = 50000.0 → "50000"
        assert_eq!(rp.px, "50000");
        // ts_ms is whole-millis present and positive
        assert!(rp.ts_ms > 0);
        // Round-trip serialization preserves px as a JSON string.
        let json = serde_json::to_value(&r).expect("serialize");
        let px_field = &json["reference_price"]["px"];
        assert!(px_field.is_string(), "px must be a JSON string, got {px_field:?}");
        assert_eq!(px_field.as_str(), Some("50000"));
    }
}
