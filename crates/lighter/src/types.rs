use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MarketStatsMsg {
    pub channel: String,
    pub market_stats: MarketStats,
}

#[derive(Debug, Deserialize)]
pub struct MarketStats {
    pub market_id: u32,
    pub mark_price: String,
    pub funding_rate: String,
    pub funding_timestamp: u64,
}
