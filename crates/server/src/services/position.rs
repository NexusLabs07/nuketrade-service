//! Position-related business logic.

use perp_core::{PositionSide, UnifiedPosition, parse_f64_or_zero};
use std::collections::HashMap;

use crate::types::{MergedPositionResponse, OpenPositionsResponse, Side};

/// Service for position conversion and merging operations.
pub struct PositionService;

impl PositionService {
    /// Convert a UnifiedPosition to an OpenPositionsResponse.
    pub fn to_response(pos: &UnifiedPosition) -> OpenPositionsResponse {
        let side = match pos.side {
            PositionSide::Long => Side::Long,
            PositionSide::Short => Side::Short,
        };

        OpenPositionsResponse {
            symbol: pos.symbol.clone(),
            size: pos.size.to_string(),
            side,
            margin: pos.margin_used.to_string(),
            pnl: pos.unrealized_pnl.to_string(),
            funding: pos.cumulative_funding.to_string(),
            leverage: pos.leverage,
            liquidation_price: pos
                .liquidation_price
                .map(|p| p.to_string())
                .unwrap_or_default(),
        }
    }

    /// Convert Hyperliquid ClearinghouseState position to OpenPositionsResponse.
    pub fn from_hyperliquid_position(
        pos: &hyperliquid::apis::user::Position,
    ) -> OpenPositionsResponse {
        let size_value = parse_f64_or_zero(&pos.szi);
        let side = if size_value > 0.0 {
            Side::Long
        } else {
            Side::Short
        };

        OpenPositionsResponse {
            symbol: pos.coin.clone(),
            size: if side == Side::Short {
                (-size_value).to_string()
            } else {
                pos.szi.clone()
            },
            side,
            margin: pos.margin_used.clone(),
            pnl: pos.unrealized_pnl.clone(),
            funding: pos.cum_funding.all_time.clone(),
            leverage: pos.leverage.value,
            liquidation_price: pos.liquidation_px.clone().unwrap_or_default(),
        }
    }

    /// Convert Pacifica UserPosition to OpenPositionsResponse.
    pub fn from_pacifica_position(
        pos: &pacifica::apis::user::UserPosition,
        leverage: u32,
    ) -> OpenPositionsResponse {
        let side = match pos.side.to_lowercase().as_str() {
            "long" => Side::Long,
            _ => Side::Short,
        };

        OpenPositionsResponse {
            symbol: pos.symbol.clone(),
            size: pos.amount.clone(),
            side,
            margin: pos.margin.clone().unwrap_or_default(),
            pnl: "0".to_string(), // PnL not in position response
            funding: pos.funding.clone().unwrap_or_default(),
            leverage,
            liquidation_price: pos.liquidation_price.clone().unwrap_or_default(),
        }
    }

    /// Merge positions from multiple exchanges into a unified view.
    ///
    /// Positions are grouped by symbol, with each exchange's position
    /// stored separately in the response.
    pub fn merge_positions(
        hl_positions: Vec<OpenPositionsResponse>,
        pacifica_positions: Vec<OpenPositionsResponse>,
    ) -> Vec<MergedPositionResponse> {
        let mut positions_map: HashMap<String, MergedPositionResponse> = HashMap::new();

        // Add Hyperliquid positions
        for pos in hl_positions {
            let symbol = pos.symbol.clone();
            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol,
                    hyperliquid: None,
                    pacifica: None,
                })
                .hyperliquid = Some(pos);
        }

        // Add Pacifica positions
        for pos in pacifica_positions {
            let symbol = pos.symbol.clone();
            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol,
                    hyperliquid: None,
                    pacifica: None,
                })
                .pacifica = Some(pos);
        }

        positions_map.into_values().collect()
    }
}
