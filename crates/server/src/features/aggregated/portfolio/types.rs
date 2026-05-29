use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    Day,
    Week,
    Month,
    All,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceBucket {
    pub volume_usd: f64,
    pub strategies_opened: i64,
    pub pnl_usd: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PerformanceResponse {
    pub day: PerformanceBucket,
    pub week: PerformanceBucket,
    pub month: PerformanceBucket,
    pub all: PerformanceBucket,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlChartPoint {
    pub timestamp: String,
    pub cumulative_pnl_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlChartResponse {
    pub timeframe: Timeframe,
    pub range_start: String,
    pub range_end: String,
    pub points: Vec<PnlChartPoint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRow {
    pub venue: &'static str,
    pub display_name: &'static str,
    pub connected: bool,
    pub available_balance_usd: Option<f64>,
    pub total_equity_usd: Option<f64>,
    /// Lifetime notional traded on this venue (|size × price|). `null` when not connected.
    pub volume_usd: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeTotals {
    pub available_balance_usd: f64,
    pub total_equity_usd: f64,
    pub volume_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExchangesResponse {
    pub exchanges: Vec<ExchangeRow>,
    pub totals: ExchangeTotals,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_shape() {
        let v = PerformanceResponse {
            day: PerformanceBucket {
                volume_usd: 1.5,
                strategies_opened: 2,
                pnl_usd: -3.0,
            },
            ..Default::default()
        };
        println!("PERF {}", serde_json::to_string(&v).unwrap());
    }

    #[test]
    fn pnl_shape() {
        let v = PnlChartResponse {
            timeframe: Timeframe::Day,
            range_start: "S".into(),
            range_end: "E".into(),
            points: vec![PnlChartPoint {
                timestamp: "T".into(),
                cumulative_pnl_usd: 4.2,
            }],
        };
        println!("PNL {}", serde_json::to_string(&v).unwrap());
    }

    #[test]
    fn ex_shape() {
        let v = ExchangesResponse {
            exchanges: vec![
                ExchangeRow {
                    venue: "hyperliquid",
                    display_name: "Hyperliquid",
                    connected: true,
                    available_balance_usd: Some(1.0),
                    total_equity_usd: Some(2.0),
                    volume_usd: Some(100.0),
                    error: None,
                },
                ExchangeRow {
                    venue: "backpack",
                    display_name: "Backpack",
                    connected: false,
                    available_balance_usd: None,
                    total_equity_usd: None,
                    volume_usd: None,
                    error: Some("not_implemented".into()),
                },
            ],
            totals: ExchangeTotals {
                available_balance_usd: 1.0,
                total_equity_usd: 2.0,
                volume_usd: 100.0,
            },
        };
        println!("EX {}", serde_json::to_string(&v).unwrap());
    }
}
