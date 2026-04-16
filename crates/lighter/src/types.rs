use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MarketStatsMsg {
    pub channel: String,
    pub timestamp: Option<i64>,
    pub market_stats: MarketStats,
    #[serde(rename = "type")]
    pub lighter_type: String,
}

#[derive(Debug, Deserialize)]
pub struct MarketStats {
    pub symbol: String,
    pub market_id: u32,
    pub mark_price: String,
    pub current_funding_rate: Option<String>,
    pub funding_rate: Option<String>,
    pub funding_timestamp: Option<i64>,
}
