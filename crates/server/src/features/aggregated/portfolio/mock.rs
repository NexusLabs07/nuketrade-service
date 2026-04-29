//! Deterministic mock data for the portfolio endpoints.
//!
//! Triggered by `?mock=true` on any portfolio endpoint. Returns realistic
//! shapes so the frontend can develop against populated UI states without
//! depending on a wallet that has actual on-chain activity.

use chrono::{DateTime, Duration, Utc};

use crate::features::aggregated::portfolio::types::{
    ExchangeRow, ExchangeTotals, ExchangesResponse, PerformanceBucket, PerformanceResponse,
    PnlChartPoint, PnlChartResponse, Timeframe,
};

pub fn performance() -> PerformanceResponse {
    PerformanceResponse {
        day: PerformanceBucket {
            volume_usd: 352.40,
            strategies_opened: 1,
            pnl_usd: 4.20,
        },
        week: PerformanceBucket {
            volume_usd: 1_840.50,
            strategies_opened: 3,
            pnl_usd: -8.75,
        },
        month: PerformanceBucket {
            volume_usd: 7_420.30,
            strategies_opened: 7,
            pnl_usd: 42.60,
        },
        all: PerformanceBucket {
            volume_usd: 28_500.00,
            strategies_opened: 18,
            pnl_usd: 156.30,
        },
    }
}

pub fn pnl_chart(timeframe: Timeframe) -> PnlChartResponse {
    let now = Utc::now();
    let (range_start, bucket_minutes, bucket_count): (DateTime<Utc>, i64, usize) = match timeframe {
        Timeframe::Day => (now - Duration::days(1), 60, 24),
        Timeframe::Week => (now - Duration::days(7), 6 * 60, 28),
        Timeframe::Month => (now - Duration::days(30), 24 * 60, 30),
        Timeframe::All => (now - Duration::days(180), 7 * 24 * 60, 26),
    };

    // Deterministic, smooth-ish curve: combine a slight upward drift with a
    // sine wave, scaled per timeframe. Values are in USD.
    let (drift_per_step, amplitude) = match timeframe {
        Timeframe::Day => (0.20, 1.5),
        Timeframe::Week => (0.80, 5.0),
        Timeframe::Month => (3.0, 15.0),
        Timeframe::All => (10.0, 50.0),
    };

    let bucket = Duration::minutes(bucket_minutes);
    let mut points = Vec::with_capacity(bucket_count);
    let mut cumulative = 0.0_f64;
    for i in 1..=bucket_count {
        let t = i as f64;
        let step = drift_per_step + amplitude * ((t / 4.0).sin());
        cumulative += step;
        // Round to 2 decimal places for cleaner mock display.
        let rounded = (cumulative * 100.0).round() / 100.0;
        let bucket_end = range_start + bucket * (i as i32);
        points.push(PnlChartPoint {
            timestamp: bucket_end.to_rfc3339(),
            cumulative_pnl_usd: rounded,
        });
    }

    PnlChartResponse {
        timeframe,
        range_start: range_start.to_rfc3339(),
        range_end: now.to_rfc3339(),
        points,
    }
}

pub fn exchanges() -> ExchangesResponse {
    let exchanges = vec![
        ExchangeRow {
            venue: "hyperliquid",
            display_name: "Hyperliquid",
            connected: true,
            available_balance_usd: Some(85.40),
            total_equity_usd: Some(92.15),
            error: None,
        },
        ExchangeRow {
            venue: "pacifica",
            display_name: "Pacifica",
            connected: true,
            available_balance_usd: Some(42.20),
            total_equity_usd: Some(44.80),
            error: None,
        },
        ExchangeRow {
            venue: "lighter",
            display_name: "Lighter",
            connected: true,
            available_balance_usd: Some(25.00),
            total_equity_usd: Some(26.10),
            error: None,
        },
        ExchangeRow {
            venue: "backpack",
            display_name: "Backpack",
            connected: true,
            available_balance_usd: Some(0.0),
            total_equity_usd: Some(0.0),
            error: None,
        },
    ];

    let mut totals = ExchangeTotals::default();
    for row in &exchanges {
        if let Some(v) = row.available_balance_usd {
            totals.available_balance_usd += v;
        }
        if let Some(v) = row.total_equity_usd {
            totals.total_equity_usd += v;
        }
    }

    ExchangesResponse { exchanges, totals }
}
