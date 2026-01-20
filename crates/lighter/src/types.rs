use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MarketStatsMsg {
    pub channel: String,
    pub market_stats: MarketStats,
    #[serde(rename = "type")]
    pub lighter_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MarketStats {
    pub market_id: u32,
    pub mark_price: String,
    pub funding_rate: String,
    pub funding_timestamp: u64,
}
