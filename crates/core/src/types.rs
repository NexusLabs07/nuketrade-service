use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformsFundingRate {
    pub hyperliquid: Option<f64>,
    pub lighter: Option<f64>,
}
