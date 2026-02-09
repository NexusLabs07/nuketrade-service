use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;

use crate::perp_metadata::PERP_META;

#[derive(Debug, Clone, Deserialize)]
pub struct PerpAsset {
    pub name: String,
    #[serde(rename = "maxLeverage")]
    pub max_leverage: u32,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
    #[serde(rename = "marginTableId")]
    pub margin_table_id: u32,
    #[serde(rename = "isDelisted", default)]
    pub is_delisted: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct PerpMetaInner {
    universe: Vec<PerpAsset>,
}

pub static HL_MARKETS: Lazy<Vec<PerpAsset>> = Lazy::new(|| {
    // Parse as generic JSON array first since it contains heterogeneous elements
    let parsed: Vec<Value> = match serde_json::from_str(PERP_META) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to parse PERP_META as array: {e}");
        }
    };

    // Only the first element contains the universe data
    let first_element = parsed
        .into_iter()
        .next()
        .expect("PERP_META must contain at least one element");

    let meta: PerpMetaInner = match serde_json::from_value(first_element) {
        Ok(m) => m,
        Err(e) => {
            panic!("Failed to parse universe from PERP_META: {e}");
        }
    };

    meta.universe
        .into_iter()
        .filter(|asset| !asset.is_delisted)
        .collect()
});

pub fn get_max_leverage(symbol: &str) -> Option<u32> {
    HL_MARKETS
        .iter()
        .find(|asset| asset.name == symbol)
        .map(|asset| asset.max_leverage)
}
