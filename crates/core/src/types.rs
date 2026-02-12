use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Dex;

/// Internal loop structure: symbol -> (mark_px, funding_rate)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RawMarketData {
    pub hyperliquid: HashMap<String, (f64, f64)>,
    pub lighter: HashMap<String, (f64, f64)>,
    pub pacifica: HashMap<String, (f64, f64)>,
}

/// Backward compatibility alias
pub type LiveMarketFeed = RawMarketData;

/// Message sent from WS tasks -> FeedManager over bounded mpsc
#[derive(Clone, Debug)]
pub struct MarketFeedUpdate {
    pub dex: Dex,
    pub data: HashMap<String, (f64, f64)>,
}
