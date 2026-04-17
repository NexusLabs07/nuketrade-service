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

#[derive(Debug, Clone, Deserialize)]
pub struct OrderBookDetailsResponse {
    pub code: u32,
    #[serde(default)]
    pub order_book_details: Vec<LighterOrderBookDetail>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LighterOrderBookDetail {
    pub symbol: String,
    pub market_id: u32,
    pub market_type: String,
    pub status: String,
    pub min_base_amount: String,
    pub supported_size_decimals: u32,
    pub supported_price_decimals: u32,
    pub size_decimals: u32,
    pub price_decimals: u32,
    pub min_initial_margin_fraction: u32,
}
