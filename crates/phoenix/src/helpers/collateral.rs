use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::apis::user::{PhoenixTrader, TraderStateResponse};

/// Matches FE `DEFAULT_TRADER_PDA_INDEX`.
pub const DEFAULT_TRADER_PDA_INDEX: u32 = 0;
/// Matches FE `DEFAULT_TRADER_SUBACCOUNT_INDEX`.
pub const DEFAULT_TRADER_SUBACCOUNT_INDEX: u32 = 0;

pub const USDC_MICROS_PER_USD: f64 = 1_000_000.0;

/// USDC amount from Phoenix REST (`{ value, decimals, ui }`) or legacy string (micros).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixAmount {
    pub value: i64,
    pub decimals: u8,
    pub ui: String,
}

impl PhoenixAmount {
    /// Parse amount using `ui` when present, else `value / 10^decimals` (signed).
    pub fn to_f64(&self) -> f64 {
        if let Ok(ui) = self.ui.trim().parse::<f64>() {
            if ui.is_finite() {
                return ui;
            }
        }

        if self.decimals > 0 {
            let denom = 10_f64.powi(self.decimals as i32);
            if denom > 0.0 {
                return self.value as f64 / denom;
            }
        }

        self.value as f64
    }

    /// Non-negative USD-style amount (collateral fields).
    pub fn to_usd(&self) -> f64 {
        self.to_f64().max(0.0)
    }
}

/// Convert raw collateral string (Rise snapshot style) to USD — micros first, then ÷ 1e6.
pub fn phoenix_collateral_raw_to_usd(raw: &str) -> f64 {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    if let Ok(micros) = trimmed.parse::<u64>() {
        return micros as f64 / USDC_MICROS_PER_USD;
    }

    if let Ok(v) = trimmed.parse::<f64>() {
        if v.is_finite() {
            return v / USDC_MICROS_PER_USD;
        }
    }

    0.0
}

impl<'de> Deserialize<'de> for PhoenixAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PhoenixAmountVisitor;

        impl<'de> Visitor<'de> for PhoenixAmountVisitor {
            type Value = PhoenixAmount;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Phoenix amount object or collateral string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let usd = phoenix_collateral_raw_to_usd(v);
                Ok(PhoenixAmount {
                    value: (usd * USDC_MICROS_PER_USD).round() as i64,
                    decimals: 6,
                    ui: format!("{usd:.6}"),
                })
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut value = 0_i64;
                let mut decimals = 6_u8;
                let mut ui = String::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "value" => value = map.next_value()?,
                        "decimals" => decimals = map.next_value()?,
                        "ui" => ui = map.next_value()?,
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(PhoenixAmount {
                    value,
                    decimals,
                    ui,
                })
            }
        }

        deserializer.deserialize_any(PhoenixAmountVisitor)
    }
}

impl PhoenixTrader {
    /// Deposited collateral for this subaccount (matches FE Rise `sub.collateral`).
    pub fn deposited_collateral_usd(&self) -> f64 {
        self.collateral_balance.to_usd()
    }

    pub fn effective_collateral_usd(&self) -> f64 {
        self.effective_collateral.to_usd()
    }

    pub fn withdrawable_collateral_usd(&self) -> f64 {
        self.effective_collateral_for_withdrawals.to_usd()
    }
}

impl TraderStateResponse {
    /// Sum deposited collateral for the default subaccount (FE: subaccount 0).
    pub fn deposited_collateral_usd_for_subaccount(&self, subaccount_index: u32) -> f64 {
        self.traders
            .iter()
            .filter(|t| t.trader_subaccount_index == subaccount_index)
            .map(PhoenixTrader::deposited_collateral_usd)
            .sum()
    }

    /// Best-effort free collateral for hedge funding skip (withdrawable → effective → deposited).
    pub fn free_collateral_usd_for_subaccount(&self, subaccount_index: u32) -> f64 {
        let traders: Vec<_> = self
            .traders
            .iter()
            .filter(|t| t.trader_subaccount_index == subaccount_index)
            .collect();

        if traders.is_empty() {
            return 0.0;
        }

        let withdrawable: f64 = traders
            .iter()
            .map(|t| t.withdrawable_collateral_usd())
            .sum();
        if withdrawable > 0.0 {
            return withdrawable;
        }

        let effective: f64 = traders.iter().map(|t| t.effective_collateral_usd()).sum();
        if effective > 0.0 {
            return effective;
        }

        traders
            .iter()
            .map(|t| t.deposited_collateral_usd())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_object_amount() {
        let raw = r#"{"value":12345678,"decimals":6,"ui":"12.345678"}"#;
        let amt: PhoenixAmount = serde_json::from_str(raw).unwrap();
        assert!((amt.to_usd() - 12.345678).abs() < 1e-6);
    }

    #[test]
    fn deserializes_micros_string() {
        let raw = r#""50000000""#;
        let amt: PhoenixAmount = serde_json::from_str(raw).unwrap();
        assert!((amt.to_usd() - 50.0).abs() < 1e-6);
    }

    #[test]
    fn phoenix_collateral_raw_to_usd_micros() {
        assert!((phoenix_collateral_raw_to_usd("50000000") - 50.0).abs() < 1e-6);
    }

    #[test]
    fn deserializes_phoenix_position_from_rest() {
        use crate::apis::user::PhoenixPosition;

        let raw = r#"{
            "symbol": "ENA",
            "positionSize": { "value": 186, "decimals": 0, "ui": "186" },
            "virtualQuotePosition": { "value": -17807640, "decimals": 6, "ui": "-17.807640" },
            "entryPrice": { "value": 95740, "decimals": 6, "ui": "0.095740" },
            "unrealizedPnl": { "value": 26040, "decimals": 6, "ui": "0.026040" },
            "positionInitialMargin": { "value": 1783368, "decimals": 6, "ui": "1.783368" },
            "initialMargin": { "value": 1783368, "decimals": 6, "ui": "1.783368" },
            "liquidationPrice": { "value": 30080, "decimals": 6, "ui": "0.030080" },
            "unsettledFunding": { "value": 1116, "decimals": 6, "ui": "0.001116" },
            "accumulatedFunding": { "value": 0, "decimals": 6, "ui": "0.000000" }
        }"#;

        let pos: PhoenixPosition = serde_json::from_str(raw).unwrap();
        assert_eq!(pos.symbol, "ENA");
        assert!((pos.signed_position_size() - 186.0).abs() < f64::EPSILON);
        assert!((pos.margin_usd() - 1.783368).abs() < 1e-4);
    }

    #[test]
    fn display_margin_matches_phoenix_ui_isolated_collateral() {
        use crate::apis::user::{PhoenixPosition, PhoenixTrader};

        let pos: PhoenixPosition = serde_json::from_str(
            r#"{
            "symbol": "JUP",
            "positionSize": { "value": 137, "decimals": 0, "ui": "137" },
            "virtualQuotePosition": { "value": -27672630, "decimals": 6, "ui": "-27.672630" },
            "entryPrice": { "value": 202150, "decimals": 6, "ui": "0.202150" },
            "positionValue": { "value": 27672630, "decimals": 6, "ui": "27.672630" },
            "positionInitialMargin": { "value": 2767263, "decimals": 6, "ui": "2.767263" },
            "unrealizedPnl": { "value": -36990, "decimals": 6, "ui": "-0.036990" }
        }"#,
        )
        .unwrap();

        let trader: PhoenixTrader = serde_json::from_str(
            r#"{
            "authority": "3v8sLhz4KfVeBroMVBUzHfYFE8g9E6pgrnUKXZnwgqEZ",
            "traderKey": "x",
            "traderPdaIndex": 0,
            "traderSubaccountIndex": 1,
            "collateralBalance": { "value": 6921276, "decimals": 6, "ui": "6.921276" },
            "positions": []
        }"#,
        )
        .unwrap();

        let collateral = trader.collateral_usd();
        assert!((collateral - 6.921276).abs() < 1e-4);
        assert!((pos.margin_usd() - 2.767263).abs() < 1e-4);
        assert!((pos.display_margin_usd(collateral, true) - 6.921276).abs() < 1e-4);
        assert_eq!(pos.display_leverage(collateral, true), 4);
    }

    #[test]
    fn infers_short_when_pnl_positive_and_mark_below_entry() {
        use crate::apis::user::PhoenixPosition;

        let raw = r#"{
            "symbol": "ENA",
            "positionSize": { "value": 100, "decimals": 0, "ui": "100" },
            "virtualQuotePosition": { "value": -1000000, "decimals": 6, "ui": "-1.000000" },
            "entryPrice": { "value": 100000, "decimals": 6, "ui": "0.100000" },
            "unrealizedPnl": { "value": 50000, "decimals": 6, "ui": "0.050000" },
            "positionValue": { "value": 950000, "decimals": 6, "ui": "0.950000" }
        }"#;

        let pos: PhoenixPosition = serde_json::from_str(raw).unwrap();
        assert!(pos.signed_position_size() < 0.0);
    }
}
