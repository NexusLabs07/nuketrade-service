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

impl PerpAsset {
    /// App/display symbol. HIP-3 assets are named like `xyz:SPCX` on Hyperliquid,
    /// but the rest of the service uses base symbols like `SPCX`.
    pub fn display_symbol(&self) -> &str {
        self.name
            .rsplit_once(':')
            .map(|(_, symbol)| symbol)
            .unwrap_or(&self.name)
    }

    /// Exact Hyperliquid coin name to use in API/WebSocket subscriptions.
    pub fn exchange_coin(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PerpMetaInner {
    universe: Vec<PerpAsset>,
}

pub static HL_MARKETS: Lazy<Vec<PerpAsset>> = Lazy::new(|| {
    let parsed: Vec<Value> = match serde_json::from_str(PERP_META) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to parse PERP_META as array: {e}");
        }
    };

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

pub fn normalize_hl_symbol(symbol: &str) -> String {
    symbol
        .rsplit_once(':')
        .map(|(_, base)| base)
        .unwrap_or(symbol)
        .to_string()
}

pub fn find_asset(symbol: &str) -> Option<&'static PerpAsset> {
    let normalized = normalize_hl_symbol(symbol);

    HL_MARKETS.iter().find(|asset| {
        asset.name == symbol
            || asset.exchange_coin() == symbol
            || asset.display_symbol() == normalized
    })
}

pub fn subscription_coin(symbol: &str) -> String {
    find_asset(symbol)
        .map(|asset| asset.exchange_coin().to_string())
        .unwrap_or_else(|| symbol.to_string())
}

pub fn get_max_leverage(symbol: &str) -> Option<u32> {
    find_asset(symbol).map(|asset| asset.max_leverage)
}
