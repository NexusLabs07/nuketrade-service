use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPositionsResponse {
    pub symbol: String,
    pub size: String,
    pub side: Side,
    pub pnl: String,
    pub funding: String,
    pub margin: String,
    pub leverage: u32,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergedPositionResponse {
    pub symbol: String,
    pub hyperliquid: Option<OpenPositionsResponse>,
    pub pacifica: Option<OpenPositionsResponse>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MarketFeedValueStruct {
    pub mark_px: Option<f64>,
    pub funding: Option<f64>,
    pub max_leverage: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMarketFeedResponse {
    pub symbol: String,
    pub hyperliquid: Option<MarketFeedValueStruct>,
    pub pacifica: Option<MarketFeedValueStruct>,
    pub backpack: Option<MarketFeedValueStruct>,
}

#[derive(Clone, Debug)]
pub struct FeedSnapshot {
    pub by_symbol: HashMap<String, LiveMarketFeedResponse>,
    pub formatted: Vec<LiveMarketFeedResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedPositionResponse {
    pub symbol: String,
    pub size: String,
    pub side: Side,
    pub pnl: String,
    #[serde(rename = "entryPrice")]
    pub entry_price: String,
    #[serde(rename = "exitPrice")]
    pub exit_price: String,
    #[serde(rename = "closedAt")]
    pub closed_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergedClosedPositionResponse {
    pub symbol: String,
    #[serde(rename = "closedAt")]
    pub closed_at: i64,
    pub hyperliquid: Option<ClosedPositionResponse>,
    pub pacifica: Option<ClosedPositionResponse>,
}
