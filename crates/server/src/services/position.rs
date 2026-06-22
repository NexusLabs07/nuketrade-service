//! Position-related business logic.

use chrono::{DateTime, Utc};
use hyperliquid::apis::user::UserFill;
use pacifica::apis::user::AccountSetting;
use perp_core::{PositionSide, UnifiedPosition, parse_f64_or_zero};
use phoenix::apis::user::PhoenixTrade;
use std::collections::HashMap;

fn pacifica_symbol_norm_base(symbol: &str) -> String {
    symbol
        .trim()
        .split('-')
        .next()
        .unwrap_or(symbol)
        .trim()
        .to_ascii_uppercase()
}

// fn json_value_to_f64(value: &serde_json::Value) -> f64 {
//     match value {
//         serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
//         serde_json::Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
//         _ => 0.0,
//     }
// }

// fn json_value_to_i64(value: &serde_json::Value) -> i64 {
//     match value {
//         serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
//         serde_json::Value::String(s) => s.parse::<i64>().unwrap_or(0),
//         _ => 0,
//     }
// }

// replace existing types import
use crate::types::{
    ClosedPositionResponse, MergedClosedPositionResponse, MergedPositionResponse,
    OpenPositionsResponse, Side,
};

/// Minimum hours held when annualizing realized funding (avoids blow-ups on sub-hour positions).
const REALIZED_FUNDING_MIN_HOURS: f64 = 1.0;
const HOURS_PER_YEAR: f64 = 24.0 * 365.0;

/// Service for position conversion and merging operations.
pub struct PositionService;

impl PositionService {
    /// Resolve user leverage from `/account/settings` for a position symbol.
    ///
    /// Pacifica may use the same base symbol with different suffixes (e.g. `JUP` vs `JUP-PERP`).
    /// When nothing matches, returns `0` — do not substitute market max leverage; that is not the
    /// user's selected leverage.
    pub fn resolve_pacifica_leverage(
        settings: Option<&[AccountSetting]>,
        position_symbol: &str,
    ) -> u32 {
        let Some(settings) = settings else {
            return 0;
        };
        let pos_full = position_symbol.trim().to_ascii_uppercase();
        let pos_base = pacifica_symbol_norm_base(position_symbol);

        for row in settings {
            let set_full = row.symbol.trim().to_ascii_uppercase();
            if set_full == pos_full {
                return row.leverage as u32;
            }
            if pacifica_symbol_norm_base(&row.symbol) == pos_base {
                return row.leverage as u32;
            }
        }
        0
    }

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
            opened_at: None,
        }
    }
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
            // HL cumFunding is inverted vs trader cashflow (+ = received, − = paid).
            funding: (-parse_f64_or_zero(&pos.cum_funding.all_time)).to_string(),
            leverage: pos.leverage.value,
            liquidation_price: pos.liquidation_px.clone().unwrap_or_default(),
            opened_at: None,
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
            opened_at: Some(pos.created_at as i64),
        }
    }

    pub fn from_phoenix_position(
        pos: &phoenix::apis::user::PhoenixPosition,
        trader: &phoenix::apis::user::PhoenixTrader,
    ) -> Option<OpenPositionsResponse> {
        let size_value = pos.signed_position_size();

        if size_value.abs() < f64::EPSILON {
            return None;
        }

        let side = if size_value > 0.0 {
            Side::Long
        } else {
            Side::Short
        };

        let collateral = trader.collateral_usd();
        let single_position = trader.positions.len() == 1;

        Some(OpenPositionsResponse {
            symbol: phoenix::helpers::markets::normalize_phoenix_symbol(&pos.symbol),
            size: size_value.abs().to_string(),
            side,
            margin: pos
                .display_margin_usd(collateral, single_position)
                .to_string(),
            pnl: pos.unrealized_pnl.to_f64().to_string(),
            funding: pos.funding_usd().to_string(),
            leverage: pos.display_leverage(collateral, single_position),
            liquidation_price: pos.liquidation_price.to_f64().to_string(),
            opened_at: None,
        })
    }

    /// Most recent Hyperliquid fill that opened the current position cycle (flat → open).
    pub fn hyperliquid_opened_at_ms(fills: &[UserFill], coin: &str) -> Option<i64> {
        let coin = coin.trim().to_ascii_uppercase();
        let mut candidates: Vec<i64> = fills
            .iter()
            .filter(|fill| fill.coin.trim().eq_ignore_ascii_case(&coin))
            .filter(|fill| fill.dir.to_ascii_lowercase().contains("open"))
            .filter(|fill| parse_f64_or_zero(&fill.start_position).abs() < f64::EPSILON)
            .map(|fill| fill.time)
            .collect();
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        candidates.first().copied()
    }

    /// Most recent Phoenix trade that opened the current position cycle (flat → open).
    pub fn phoenix_opened_at_ms(trades: &[PhoenixTrade], symbol: &str) -> Option<i64> {
        let symbol = phoenix::helpers::markets::normalize_phoenix_symbol(symbol);
        let mut candidates: Vec<i64> = trades
            .iter()
            .filter(|trade| {
                phoenix::helpers::markets::normalize_phoenix_symbol(&trade.market_symbol) == symbol
            })
            .filter(|trade| parse_f64_or_zero(&trade.base_lots_before).abs() < f64::EPSILON)
            .filter(|trade| parse_f64_or_zero(&trade.base_lots_after).abs() >= f64::EPSILON)
            .filter_map(|trade| parse_rfc3339_ms(&trade.timestamp))
            .collect();
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        candidates.first().copied()
    }

    /// Annualized realized funding APR on combined margin for a merged hedge row.
    pub fn realized_funding_apr_pct(merged: &MergedPositionResponse, now_ms: i64) -> Option<f64> {
        let opened_at = merged.opened_at?;
        if now_ms <= opened_at {
            return None;
        }

        let mut total_funding = 0.0;
        let mut total_margin = 0.0;
        for leg in [
            merged.hyperliquid.as_ref(),
            merged.pacifica.as_ref(),
            merged.phoenix.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            total_funding += parse_f64_or_zero(&leg.funding);
            total_margin += parse_f64_or_zero(&leg.margin);
        }

        if total_margin <= f64::EPSILON {
            return None;
        }

        let hours = ((now_ms - opened_at) as f64 / 3_600_000.0).max(REALIZED_FUNDING_MIN_HOURS);
        let return_on_margin = total_funding / total_margin;
        Some(return_on_margin / hours * HOURS_PER_YEAR * 100.0)
    }

    /// Merge positions from multiple exchanges into a unified view.
    ///
    /// Positions are grouped by symbol, with each exchange's position
    /// stored separately in the response.
    pub fn merge_positions(
        hl_positions: Vec<OpenPositionsResponse>,
        pacifica_positions: Vec<OpenPositionsResponse>,
        phoenix_positions: Vec<OpenPositionsResponse>,
    ) -> Vec<MergedPositionResponse> {
        Self::merge_positions_at(hl_positions, pacifica_positions, phoenix_positions, Utc::now())
    }

    pub fn merge_positions_at(
        hl_positions: Vec<OpenPositionsResponse>,
        pacifica_positions: Vec<OpenPositionsResponse>,
        phoenix_positions: Vec<OpenPositionsResponse>,
        now: DateTime<Utc>,
    ) -> Vec<MergedPositionResponse> {
        let now_ms = now.timestamp_millis();
        let mut positions_map: HashMap<String, MergedPositionResponse> = HashMap::new();

        for pos in hl_positions {
            let symbol = pos.symbol.clone();
            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                    opened_at: None,
                    realized_funding_apr: None,
                })
                .hyperliquid = Some(pos);
        }

        for pos in pacifica_positions {
            let symbol = pos.symbol.clone();
            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                    opened_at: None,
                    realized_funding_apr: None,
                })
                .pacifica = Some(pos);
        }

        for pos in phoenix_positions {
            let symbol = pos.symbol.clone();
            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                    opened_at: None,
                    realized_funding_apr: None,
                })
                .phoenix = Some(pos);
        }

        let mut merged: Vec<MergedPositionResponse> = positions_map.into_values().collect();
        for row in &mut merged {
            row.opened_at = latest_leg_opened_at(row);
            row.realized_funding_apr = Self::realized_funding_apr_pct(row, now_ms);
        }
        merged
    }

    pub fn from_pacifica_position_with_metrics(
        pos: &pacifica::apis::user::UserPosition,
        leverage: u32,
        margin: String,
        pnl: f64,
    ) -> OpenPositionsResponse {
        let is_bid = pos.side == "bid";
        let is_ask = pos.side == "ask";

        OpenPositionsResponse {
            symbol: pos.symbol.clone(),
            size: pos.amount.clone(),
            side: if is_bid { Side::Long } else { Side::Short },
            pnl: if is_ask {
                (-pnl).to_string()
            } else {
                pnl.to_string()
            },
            margin,
            funding: pos.funding.clone().unwrap_or_default(),
            leverage,
            liquidation_price: pos.liquidation_price.clone().unwrap_or_default(),
            opened_at: Some(pos.created_at as i64),
        }
    }

    pub fn from_hyperliquid_closed_fill(
        fill: &hyperliquid::apis::user::UserFill,
    ) -> Option<ClosedPositionResponse> {
        if fill.coin.is_empty() {
            return None;
        }

        let dir = fill.dir.to_ascii_lowercase();
        let side = if dir.contains("close long") {
            Side::Long
        } else if dir.contains("close short") {
            Side::Short
        } else {
            return None;
        };

        Some(ClosedPositionResponse {
            symbol: fill.coin.clone(),
            size: fill.sz.clone(),
            side,
            pnl: if fill.closed_pnl.is_empty() {
                "0".to_string()
            } else {
                fill.closed_pnl.clone()
            },
            entry_price: String::new(),
            exit_price: fill.px.clone(),
            closed_at: fill.time,
        })
    }

    pub fn from_pacifica_closed_position(
        pos: &pacifica::apis::user::UserPositionHistory,
    ) -> Option<ClosedPositionResponse> {
        if pos.symbol.is_empty() {
            return None;
        }

        let status = pos.status.to_ascii_lowercase();
        if !status.is_empty()
            && !matches!(
                status.as_str(),
                "filled" | "closed" | "executed" | "success"
            )
        {
            return None;
        }

        let side_raw = pos.side.to_ascii_lowercase();
        let side = match side_raw.as_str() {
            "close_long" | "close long" | "close-long" => Side::Long,
            "close_short" | "close short" | "close-short" => Side::Short,
            _ => return None,
        };

        Some(ClosedPositionResponse {
            symbol: pos.symbol.clone(),
            size: pos.amount.clone(),
            side,
            pnl: "0".to_string(),
            entry_price: String::new(),
            exit_price: pos.execution_price.clone(),
            closed_at: pos.timestamp,
        })
    }

    pub fn from_phoenix_closed_trade(
        trade: &phoenix::apis::user::PhoenixTrade,
    ) -> Option<ClosedPositionResponse> {
        if trade.market_symbol.is_empty() {
            return None;
        }

        let pnl = trade.realized_pnl.parse::<f64>().unwrap_or(0.0);
        if pnl.abs() < f64::EPSILON {
            return None;
        }

        let size = trade.base_lots_delta.parse::<f64>().unwrap_or(0.0);
        let side = if size > 0.0 { Side::Long } else { Side::Short };

        let closed_at = chrono::DateTime::parse_from_rfc3339(&trade.timestamp)
            .map(|t| t.timestamp_millis())
            .unwrap_or(0);

        Some(ClosedPositionResponse {
            symbol: phoenix::helpers::markets::normalize_phoenix_symbol(&trade.market_symbol),
            size: size.abs().to_string(),
            side,
            pnl: pnl.to_string(),
            entry_price: String::new(),
            exit_price: trade.price.clone(),
            closed_at,
        })
    }

    pub fn merge_closed_positions(
        hl_positions: Vec<ClosedPositionResponse>,
        pacifica_positions: Vec<ClosedPositionResponse>,
        phoenix_positions: Vec<ClosedPositionResponse>,
    ) -> Vec<MergedClosedPositionResponse> {
        let mut positions_map: HashMap<(String, i64), MergedClosedPositionResponse> =
            HashMap::new();

        for pos in hl_positions {
            let symbol = pos.symbol.clone();
            let closed_at = pos.closed_at;

            let entry = positions_map.entry((symbol.clone(), closed_at)).or_insert(
                MergedClosedPositionResponse {
                    symbol,
                    closed_at,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                },
            );

            entry.hyperliquid = Some(pos);
        }

        for pos in pacifica_positions {
            let symbol = pos.symbol.clone();
            let closed_at = pos.closed_at;

            let entry = positions_map.entry((symbol.clone(), closed_at)).or_insert(
                MergedClosedPositionResponse {
                    symbol,
                    closed_at,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                },
            );

            entry.pacifica = Some(pos);
        }

        for pos in phoenix_positions {
            let symbol = pos.symbol.clone();
            let closed_at = pos.closed_at;

            let entry = positions_map.entry((symbol.clone(), closed_at)).or_insert(
                MergedClosedPositionResponse {
                    symbol,
                    closed_at,
                    hyperliquid: None,
                    pacifica: None,
                    phoenix: None,
                },
            );

            entry.phoenix = Some(pos);
        }

        let mut merged_positions: Vec<MergedClosedPositionResponse> =
            positions_map.into_values().collect();
        merged_positions.sort_by(|a, b| b.closed_at.cmp(&a.closed_at));
        merged_positions
    }
}

fn latest_leg_opened_at(merged: &MergedPositionResponse) -> Option<i64> {
    [
        merged.hyperliquid.as_ref().and_then(|p| p.opened_at),
        merged.pacifica.as_ref().and_then(|p| p.opened_at),
        merged.phoenix.as_ref().and_then(|p| p.opened_at),
    ]
    .into_iter()
    .flatten()
    .max()
}

fn parse_rfc3339_ms(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn hyperliquid_opened_at_uses_latest_flat_to_open_fill() {
        let fills = vec![
            UserFill {
                coin: "MON".into(),
                dir: "Open Short".into(),
                side: String::new(),
                px: String::new(),
                sz: String::new(),
                closed_pnl: String::new(),
                start_position: "0.0".into(),
                time: 1000,
            },
            UserFill {
                coin: "MON".into(),
                dir: "Open Short".into(),
                side: String::new(),
                px: String::new(),
                sz: String::new(),
                closed_pnl: String::new(),
                start_position: "0.0".into(),
                time: 2000,
            },
        ];
        assert_eq!(PositionService::hyperliquid_opened_at_ms(&fills, "MON"), Some(2000));
    }

    #[test]
    fn realized_funding_apr_annualizes_on_margin() {
        let merged = MergedPositionResponse {
            symbol: "MON".into(),
            hyperliquid: Some(OpenPositionsResponse {
                symbol: "MON".into(),
                size: "1".into(),
                side: Side::Short,
                pnl: "0".into(),
                funding: "0.10".into(),
                margin: "500".into(),
                leverage: 3,
                liquidation_price: "0".into(),
                opened_at: Some(0),
            }),
            pacifica: None,
            phoenix: Some(OpenPositionsResponse {
                symbol: "MON".into(),
                size: "1".into(),
                side: Side::Long,
                pnl: "0".into(),
                funding: "0.08".into(),
                margin: "500".into(),
                leverage: 3,
                liquidation_price: "0".into(),
                opened_at: Some(0),
            }),
            opened_at: Some(0),
            realized_funding_apr: None,
        };

        let now = Utc.timestamp_millis_opt(3_600_000).unwrap();
        let apr = PositionService::realized_funding_apr_pct(&merged, now.timestamp_millis()).unwrap();
        // 0.18 / 1000 margin over 1h -> 0.00018/hr on margin -> *8760*100 ≈ 157.68%
        assert!((apr - 157.68).abs() < 0.1);
    }
}
