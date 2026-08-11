use perp_core::SevenDayApr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MarketFeedValueStruct {
    pub mark_px: Option<f64>,
    pub funding: Option<f64>,
    pub max_leverage: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMarketFeedResponse {
    pub symbol: String,
    pub bulk: Option<MarketFeedValueStruct>,
    pub hyperliquid: Option<MarketFeedValueStruct>,
    pub pacifica: Option<MarketFeedValueStruct>,
    pub phoenix: Option<MarketFeedValueStruct>,
    // [backpack/lighter disabled]
    // pub backpack: Option<MarketFeedValueStruct>,
    // pub lighter: Option<MarketFeedValueStruct>,
}

#[derive(Clone, Debug)]
pub struct FeedSnapshot {
    pub by_symbol: HashMap<String, LiveMarketFeedResponse>,
    pub formatted: Vec<LiveMarketFeedResponse>,
}

/// Snapshot served to the TypeScript API over /internal/feed/snapshot.
#[derive(Debug, Serialize)]
pub struct FeedSnapshotResponse {
    pub feed: Vec<LiveMarketFeedResponse>,
    pub seven_day_apr: SevenDayApr,
}
