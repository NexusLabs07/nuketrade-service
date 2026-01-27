use serde::{Deserialize, Serialize};

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
