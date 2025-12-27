use core::fmt;

#[derive(Debug, Clone)]
pub enum Dex {
    Hyperliquid,
    Lighter,
}

impl fmt::Display for Dex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Dex::Hyperliquid => "hyperliquid",
            Dex::Lighter => "lighter",
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct FundingSnapshot {
    pub dex: Dex,
    pub coin: String,
    pub funding_hr: f64,
    pub mark_price: f64,
    pub timestamp_ms: i64,
}
