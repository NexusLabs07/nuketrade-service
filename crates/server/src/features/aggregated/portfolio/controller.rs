use anyhow::Result;
use axum::{Json, extract::State};
use chrono::{DateTime, Duration, Utc};
use hyperliquid::apis::user::{ClearinghouseState, UserFill, UserInfo as HyperliquidUserInfo};
use lighter::apis::user::{AccountByL1Response, UserInfo as LighterUserInfo};
use pacifica::apis::user::{
    AccountInfoResponse, TradeHistoryResponse, UserInfo as PacificaUserInfo,
};
use phoenix::apis::user::{
    TradeHistoryResponse as PhoenixTradeHistoryResponse, UserInfo as PhoenixUserInfo,
};
use serde::Deserialize;
use sqlx::Row;
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedPath, ValidatedQuery},
    features::aggregated::portfolio::types::{
        ExchangeRow, ExchangeTotals, ExchangesResponse, PerformanceBucket, PerformanceResponse,
        PnlChartPoint, PnlChartResponse, Timeframe,
    },
    validation::address::{validate_evm_address, validate_solana_address},
};

#[derive(Deserialize, Validate)]
pub struct PortfolioPathParams {
    #[validate(custom(function = "validate_evm_address"))]
    pub user_evm_address: String,
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

#[derive(Deserialize, Validate)]
pub struct PnlChartQuery {
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default)]
    pub tz: Option<String>,
}

fn default_timeframe() -> String {
    "day".into()
}

fn parse_timeframe(s: &str) -> Result<Timeframe, AppError> {
    match s {
        "day" => Ok(Timeframe::Day),
        "week" => Ok(Timeframe::Week),
        "month" => Ok(Timeframe::Month),
        "all" => Ok(Timeframe::All),
        _ => Err(AppError::parse(
            "timeframe",
            "must be one of: day, week, month, all",
        )),
    }
}

/// A normalized fill across venues. Used to compute volume + PnL within a window.
#[derive(Debug, Clone)]
struct NormalizedFill {
    /// UTC timestamp.
    ts: DateTime<Utc>,
    /// Notional USD value of the fill (always positive).
    notional_usd: f64,
    /// Realized PnL on this fill (may be negative).
    pnl_usd: f64,
}

fn parse_decimal(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or(0.0)
}

fn parse_phoenix_timestamp(value: &str) -> DateTime<Utc> {
    if let Ok(ts) = DateTime::parse_from_rfc3339(value) {
        return ts.with_timezone(&Utc);
    }

    if let Ok(ms) = value.parse::<i64>() {
        return DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_else(Utc::now);
    }

    Utc::now()
}

// ============================ Performance ============================
pub async fn get_performance(
    ValidatedPath(params): ValidatedPath<PortfolioPathParams>,
    State(state): State<AppState>,
) -> Result<Json<PerformanceResponse>, AppError> {
    let now = Utc::now();
    let day_start = now - Duration::days(1);
    let week_start = now - Duration::days(7);
    let month_start = now - Duration::days(30);

    let (hl_fills, pacifica_fills, phoenix_fills) = tokio::join!(
        fetch_hl_fills(&params.user_evm_address),
        fetch_pacifica_fills(&params.user_solana_address),
        fetch_phoenix_fills(&params.user_solana_address),
    );

    let mut all_fills: Vec<NormalizedFill> = Vec::new();
    all_fills.extend(hl_fills);
    all_fills.extend(pacifica_fills);
    all_fills.extend(phoenix_fills);

    let (day_count, week_count, month_count, all_count) = tokio::join!(
        count_strategies_opened(&state, &params, Some(day_start)),
        count_strategies_opened(&state, &params, Some(week_start)),
        count_strategies_opened(&state, &params, Some(month_start)),
        count_strategies_opened(&state, &params, None),
    );

    let resp = PerformanceResponse {
        day: aggregate_bucket(&all_fills, Some(day_start), now, day_count.unwrap_or(0)),
        week: aggregate_bucket(&all_fills, Some(week_start), now, week_count.unwrap_or(0)),
        month: aggregate_bucket(&all_fills, Some(month_start), now, month_count.unwrap_or(0)),
        all: aggregate_bucket(&all_fills, None, now, all_count.unwrap_or(0)),
    };

    Ok(Json(resp))
}

fn aggregate_bucket(
    fills: &[NormalizedFill],
    start: Option<DateTime<Utc>>,
    end: DateTime<Utc>,
    strategies_opened: i64,
) -> PerformanceBucket {
    let mut volume_usd = 0.0_f64;
    let mut pnl_usd = 0.0_f64;
    for f in fills {
        let after_start = start.is_none_or(|s| f.ts >= s);
        let before_end = f.ts <= end;
        if after_start && before_end {
            volume_usd += f.notional_usd;
            pnl_usd += f.pnl_usd;
        }
    }
    PerformanceBucket {
        volume_usd,
        strategies_opened,
        pnl_usd,
    }
}

// ============================ PnL chart ============================

pub async fn get_pnl_chart(
    ValidatedPath(params): ValidatedPath<PortfolioPathParams>,
    ValidatedQuery(query): ValidatedQuery<PnlChartQuery>,
    State(_state): State<AppState>,
) -> Result<Json<PnlChartResponse>, AppError> {
    let timeframe = parse_timeframe(&query.timeframe)?;
    let now = Utc::now();

    let (range_start, bucket_minutes) = match timeframe {
        Timeframe::Day => (now - Duration::days(1), 60),
        Timeframe::Week => (now - Duration::days(7), 6 * 60),
        Timeframe::Month => (now - Duration::days(30), 24 * 60),
        Timeframe::All => (now - Duration::days(365), 7 * 24 * 60),
    };

    let (hl_fills, pacifica_fills, phoenix_fills) = tokio::join!(
        fetch_hl_fills(&params.user_evm_address),
        fetch_pacifica_fills(&params.user_solana_address),
        fetch_phoenix_fills(&params.user_solana_address),
    );

    let mut all_fills: Vec<NormalizedFill> = Vec::new();
    all_fills.extend(hl_fills);
    all_fills.extend(pacifica_fills);
    all_fills.extend(phoenix_fills);

    // Effective range start: for "all", clamp to first fill if any.
    let effective_start = if matches!(timeframe, Timeframe::All) {
        all_fills.iter().map(|f| f.ts).min().unwrap_or(range_start)
    } else {
        range_start
    };

    let bucket = Duration::minutes(bucket_minutes);
    let bucket_count = ((now - effective_start).num_minutes() / bucket_minutes).max(1) as usize;

    let mut points: Vec<PnlChartPoint> = Vec::with_capacity(bucket_count);
    let mut cumulative = 0.0_f64;
    for i in 1..=bucket_count {
        let bucket_end = effective_start + bucket * (i as i32);
        let bucket_start = bucket_end - bucket;
        let bucket_pnl: f64 = all_fills
            .iter()
            .filter(|f| f.ts > bucket_start && f.ts <= bucket_end)
            .map(|f| f.pnl_usd)
            .sum();
        cumulative += bucket_pnl;
        points.push(PnlChartPoint {
            timestamp: bucket_end.to_rfc3339(),
            cumulative_pnl_usd: cumulative,
        });
    }

    Ok(Json(PnlChartResponse {
        timeframe,
        range_start: effective_start.to_rfc3339(),
        range_end: now.to_rfc3339(),
        points,
    }))
}

// ============================ Exchanges ============================

pub async fn get_exchanges(
    ValidatedPath(params): ValidatedPath<PortfolioPathParams>,
    State(_state): State<AppState>,
) -> Result<Json<ExchangesResponse>, AppError> {
    let (hl, pacifica, phoenix, lighter, backpack) = tokio::join!(
        fetch_hl_balance(&params.user_evm_address),
        fetch_pacifica_balance(&params.user_solana_address),
        fetch_phoenix_balance(&params.user_solana_address),
        fetch_lighter_balance(&params.user_evm_address),
        async {
            ExchangeRow {
                venue: "backpack",
                display_name: "Backpack",
                connected: false,
                available_balance_usd: None,
                total_equity_usd: None,
                error: Some("not_implemented".into()),
            }
        },
    );

    let exchanges = vec![hl, pacifica, phoenix, lighter, backpack];

    let mut totals = ExchangeTotals::default();
    for row in &exchanges {
        if row.connected {
            if let Some(v) = row.available_balance_usd {
                totals.available_balance_usd += v;
            }
            if let Some(v) = row.total_equity_usd {
                totals.total_equity_usd += v;
            }
        }
    }

    Ok(Json(ExchangesResponse { exchanges, totals }))
}

// ============================ Per-venue fetchers ============================

async fn fetch_hl_fills(evm_address: &str) -> Vec<NormalizedFill> {
    let client = HyperliquidUserInfo::new(Some(evm_address.to_string()), None);
    let fills: Vec<UserFill> = match client.get_closed_positions().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("hyperliquid fills fetch failed: {e}");
            return vec![];
        }
    };

    let mut out: Vec<NormalizedFill> = Vec::with_capacity(fills.len());
    for f in &fills {
        let px = f.px.parse::<f64>().unwrap_or(0.0);
        let sz = f.sz.parse::<f64>().unwrap_or(0.0);
        let pnl = f.closed_pnl.parse::<f64>().unwrap_or(0.0);
        let ts = DateTime::<Utc>::from_timestamp_millis(f.time).unwrap_or_else(Utc::now);
        out.push(NormalizedFill {
            ts,
            notional_usd: (px * sz).abs(),
            pnl_usd: pnl,
        });
    }
    out
}

async fn fetch_pacifica_fills(solana_address: &str) -> Vec<NormalizedFill> {
    let client = PacificaUserInfo::new(solana_address.to_string());
    let resp: TradeHistoryResponse = match client.get_trade_history().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("pacifica trade history fetch failed: {e}");
            return vec![];
        }
    };
    if !resp.success {
        return vec![];
    }
    let entries = resp.data.unwrap_or_default();
    let mut out: Vec<NormalizedFill> = Vec::with_capacity(entries.len());
    for t in &entries {
        let amount = t.amount.parse::<f64>().unwrap_or(0.0);
        let price = t.price.parse::<f64>().unwrap_or(0.0);
        let pnl = t.pnl.parse::<f64>().unwrap_or(0.0);
        let ts = DateTime::<Utc>::from_timestamp_millis(t.created_at).unwrap_or_else(Utc::now);
        out.push(NormalizedFill {
            ts,
            notional_usd: (amount * price).abs(),
            pnl_usd: pnl,
        });
    }
    out
}

async fn fetch_phoenix_fills(solana_address: &str) -> Vec<NormalizedFill> {
    let client = PhoenixUserInfo::new(solana_address.to_string());

    let resp: PhoenixTradeHistoryResponse = match client.get_trade_history().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("phoenix trade history fetch failed: {e}");
            return vec![];
        }
    };

    let mut out = Vec::with_capacity(resp.data.len());

    for t in &resp.data {
        let pnl = parse_decimal(&t.realized_pnl);
        let price = parse_decimal(&t.price);
        let base_lots = parse_decimal(&t.base_lots_delta).abs();
        let quote_lots = parse_decimal(&t.virtual_quote_lots_delta).abs();

        let notional_usd = if quote_lots > 0.0 {
            quote_lots
        } else {
            price * base_lots
        };

        out.push(NormalizedFill {
            ts: parse_phoenix_timestamp(&t.timestamp),
            notional_usd,
            pnl_usd: pnl,
        });
    }

    out
}

async fn fetch_hl_balance(evm_address: &str) -> ExchangeRow {
    let client = HyperliquidUserInfo::new(Some(evm_address.to_string()), None);
    let row_template = || ExchangeRow {
        venue: "hyperliquid",
        display_name: "Hyperliquid",
        connected: false,
        available_balance_usd: None,
        total_equity_usd: None,
        error: None,
    };

    let state: ClearinghouseState = match client.get_open_positions().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("hyperliquid clearinghouseState failed: {e}");
            let mut row = row_template();
            row.error = Some("upstream_unavailable".into());
            return row;
        }
    };

    let withdrawable = state.withdrawable.parse::<f64>().ok();
    let account_value = state.margin_summary.account_value.parse::<f64>().ok();
    let connected = withdrawable.unwrap_or(0.0) > 0.0
        || account_value.unwrap_or(0.0) > 0.0
        || !state.asset_positions.is_empty();
    ExchangeRow {
        venue: "hyperliquid",
        display_name: "Hyperliquid",
        connected,
        available_balance_usd: withdrawable,
        total_equity_usd: account_value,
        error: None,
    }
}

async fn fetch_pacifica_balance(solana_address: &str) -> ExchangeRow {
    let client = PacificaUserInfo::new(solana_address.to_string());
    let resp: AccountInfoResponse = match client.get_account_info().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("pacifica account info failed: {e}");
            return ExchangeRow {
                venue: "pacifica",
                display_name: "Pacifica",
                connected: false,
                available_balance_usd: None,
                total_equity_usd: None,
                error: Some("upstream_unavailable".into()),
            };
        }
    };

    if !resp.success {
        // "Account not found" is a normal disconnected state.
        return ExchangeRow {
            venue: "pacifica",
            display_name: "Pacifica",
            connected: false,
            available_balance_usd: None,
            total_equity_usd: None,
            error: None,
        };
    }

    let data = resp.data.unwrap_or_default();
    let available = data.available_to_withdraw.parse::<f64>().ok();
    let equity = data.account_equity.parse::<f64>().ok();
    ExchangeRow {
        venue: "pacifica",
        display_name: "Pacifica",
        connected: true,
        available_balance_usd: available,
        total_equity_usd: equity,
        error: None,
    }
}

async fn fetch_phoenix_balance(solana_address: &str) -> ExchangeRow {
    let client = PhoenixUserInfo::new(solana_address.to_string());

    let state = match client.get_trader_state().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("phoenix trader state failed: {e}");
            return ExchangeRow {
                venue: "phoenix",
                display_name: "Phoenix",
                connected: false,
                available_balance_usd: None,
                total_equity_usd: None,
                error: Some("upstream_unavailable".into()),
            };
        }
    };

    let mut available = 0.0_f64;
    let mut equity = 0.0_f64;
    let mut has_position = false;

    for trader in &state.traders {
        available += parse_decimal(&trader.effective_collateral_for_withdrawals);

        let trader_equity = parse_decimal(&trader.portfolio_value);
        if trader_equity > 0.0 {
            equity += trader_equity;
        } else {
            equity += parse_decimal(&trader.effective_collateral);
        }

        if !trader.positions.is_empty() {
            has_position = true;
        }
    }

    let connected = available > 0.0 || equity > 0.0 || has_position;

    ExchangeRow {
        venue: "phoenix",
        display_name: "Phoenix",
        connected,
        available_balance_usd: if connected { Some(available) } else { None },
        total_equity_usd: if connected { Some(equity) } else { None },
        error: None,
    }
}

async fn fetch_lighter_balance(evm_address: &str) -> ExchangeRow {
    let client = LighterUserInfo::new(evm_address.to_string());
    let resp: AccountByL1Response = match client.get_account_detail().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("lighter account detail failed: {e}");
            return ExchangeRow {
                venue: "lighter",
                display_name: "Lighter",
                connected: false,
                available_balance_usd: None,
                total_equity_usd: None,
                error: Some("upstream_unavailable".into()),
            };
        }
    };

    let Some(account) = resp.accounts.first() else {
        return ExchangeRow {
            venue: "lighter",
            display_name: "Lighter",
            connected: false,
            available_balance_usd: None,
            total_equity_usd: None,
            error: None,
        };
    };

    let available = account.available_balance.parse::<f64>().ok();
    let collateral = account.collateral.parse::<f64>().ok();
    let unrealized: f64 = account
        .positions
        .iter()
        .filter_map(|p| p.unrealized_pnl.parse::<f64>().ok())
        .sum();
    let equity = collateral.map(|c| c + unrealized);

    let connected = available.unwrap_or(0.0) > 0.0
        || collateral.unwrap_or(0.0) > 0.0
        || !account.positions.is_empty();

    ExchangeRow {
        venue: "lighter",
        display_name: "Lighter",
        connected,
        available_balance_usd: available,
        total_equity_usd: equity,
        error: None,
    }
}

// ============================ Hedge intent counts ============================

/// Counts hedge intents that successfully reached an opened state for this user
/// (either still ACTIVE, or already CANCELLING / CANCELLED — all imply the
/// strategy was opened at some point). Optionally filtered by created_at >= since.
async fn count_strategies_opened(
    state: &AppState,
    params: &PortfolioPathParams,
    since: Option<DateTime<Utc>>,
) -> Result<i64> {
    let sql = if since.is_some() {
        "SELECT COUNT(*) AS c FROM hedge_intents \
         WHERE evm_address = $1 AND solana_address = $2 \
         AND status IN ('ACTIVE','CANCELLING','CANCELLED') \
         AND created_at >= $3"
    } else {
        "SELECT COUNT(*) AS c FROM hedge_intents \
         WHERE evm_address = $1 AND solana_address = $2 \
         AND status IN ('ACTIVE','CANCELLING','CANCELLED')"
    };

    let mut q = sqlx::query(sql)
        .bind(&params.user_evm_address)
        .bind(&params.user_solana_address);
    if let Some(s) = since {
        q = q.bind(s.naive_utc());
    }

    let row = q.fetch_one(&*state.db).await?;
    Ok(row.try_get::<i64, _>("c").unwrap_or(0))
}
