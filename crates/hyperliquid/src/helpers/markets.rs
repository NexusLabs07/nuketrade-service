use std::collections::HashMap;

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

/// Map of perp symbol → HL asset index. Built from the UNFILTERED universe
/// because HL's order action uses positions in the original array; the
/// `HL_MARKETS` Vec drops delisted assets and would shift downstream indices.
pub static HL_ASSET_INDEX: Lazy<HashMap<String, u32>> = Lazy::new(|| {
    let parsed: Vec<Value> = serde_json::from_str(PERP_META)
        .expect("Failed to parse PERP_META as array");
    let first = parsed
        .into_iter()
        .next()
        .expect("PERP_META must contain at least one element");
    let meta: PerpMetaInner =
        serde_json::from_value(first).expect("Failed to parse universe from PERP_META");
    meta.universe
        .into_iter()
        .enumerate()
        .map(|(idx, asset)| (asset.name, idx as u32))
        .collect()
});

pub fn get_asset_index(symbol: &str) -> Option<u32> {
    HL_ASSET_INDEX.get(symbol).copied()
}

/// `szDecimals` (size precision) for an asset. Needed to format the order
/// size string to HL's exact precision; an extra digit will be rejected.
pub fn get_sz_decimals(symbol: &str) -> Option<u32> {
    HL_MARKETS
        .iter()
        .find(|asset| asset.name == symbol)
        .map(|asset| asset.sz_decimals)
}
