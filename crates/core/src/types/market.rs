//! Market metadata types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::exchange::PerpetualExchange;

/// Market metadata that varies per exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub symbol: String,
    pub max_leverage: u32,
    pub tick_size: f64,
    pub min_order_size: f64,
    pub size_decimals: u32,
    pub is_active: bool,
    /// Exchange-specific identifier (e.g., Lighter's market_index)
    pub exchange_id: Option<u32>,
}

/// Live feed to store current token price and funding rate.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RawMarketData {
    // [backpack/lighter disabled]
    // pub backpack: HashMap<String, (f64, f64)>,
    pub hyperliquid: HashMap<String, (f64, f64)>,
    // pub lighter: HashMap<String, (f64, f64)>,
    pub pacifica: HashMap<String, (f64, f64)>,
    pub risex: HashMap<String, (f64, f64)>,
}

pub type LiveMarketFeed = RawMarketData;

/// Message sent from WS tasks
#[derive(Clone, Debug)]
pub struct MarketFeedUpdate {
    pub exchange: PerpetualExchange,
    pub data: HashMap<String, (f64, f64)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SevenDayApr {
    /// symbol -> platform -> avg_rate
    pub seven_day_avg_apr: HashMap<String, HashMap<String, f64>>,
    /// Complete fixed-direction pair windows, ranked by seven-day gross return.
    /// Symbols or directed pairs without exact 168-hour coverage are omitted.
    pub seven_day_spread_apr: HashMap<String, Vec<PairSpread>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairSpread {
    pub long_platform: String,
    pub short_platform: String,
    /// Signed cumulative seven-day gross funding return in percentage points.
    /// This value is not annualized and excludes all execution costs.
    pub total_spread: f64,
}
