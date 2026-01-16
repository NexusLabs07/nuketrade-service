use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformsFundingRate {
    pub hyperliquid: HashMap<String, f64>,
    pub lighter: HashMap<String, f64>,
    pub pacifica: HashMap<String, f64>,
}
