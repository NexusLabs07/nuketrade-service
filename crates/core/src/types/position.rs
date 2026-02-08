//! Position-related types.

use serde::{Deserialize, Serialize};

/// Unified position representation across all exchanges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPosition {
    pub symbol: String,
    pub size: f64,
    pub side: PositionSide,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub cumulative_funding: f64,
    pub leverage: u32,
    pub margin_used: f64,
    pub liquidation_price: Option<f64>,
}

/// Position side enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "long"),
            PositionSide::Short => write!(f, "short"),
        }
    }
}
