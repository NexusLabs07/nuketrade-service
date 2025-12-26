#[derive(Debug, Clone)]
pub enum Dex {
    Hyperliquid,
    Lighter,
}

#[derive(Debug, Clone)]
pub struct FundingSnapshot {
    pub dex: Dex,
    pub coin: String,
    pub funding_hr: f64,
    pub mark_price: f64,
    pub timestamp_ms: i64,
}
