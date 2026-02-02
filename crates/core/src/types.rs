use std::collections::HashMap;

use serde::{Deserialize, Serialize};

//Live feed to store current token price and funding rate
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveMarketFeed {
    pub hyperliquid: HashMap<String, (f64, f64)>,
    pub lighter: HashMap<String, (f64, f64)>,
    pub pacifica: HashMap<String, (f64, f64)>,
}
