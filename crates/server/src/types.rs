use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionsResponse {
    pub symbol: String,
    pub size: String,
    pub pnl: String,
    pub funding: String,
    pub leverage: u32,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: String,
}
