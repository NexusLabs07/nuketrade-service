//! Market metadata types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveMarketFeed {
    pub hyperliquid: HashMap<String, (f64, f64)>,
    pub lighter: HashMap<String, (f64, f64)>,
    pub pacifica: HashMap<String, (f64, f64)>,
}
