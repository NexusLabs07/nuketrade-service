use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionsResponse {
    pub coin: String,
    pub size: String,
    pub pnl: String,
    pub funding: String,
    pub leverage: u32,
    pub liquidation_price: String,
}
