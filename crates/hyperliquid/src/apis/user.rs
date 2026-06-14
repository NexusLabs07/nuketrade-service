use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::HYPERLIQUID_HTTP_URL;

pub struct UserInfo {
    pub client: Client,
    pub base_url: String,
    pub evm_address: Option<String>,
    pub solana_address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FundingRateRequest {
    #[serde(rename = "type")]
    pub funding_type: String,
    pub user: String,
    #[serde(rename = "startTime")]
    pub start_time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionRequest {
    #[serde(rename = "type")]
    pub position_type: String,
    pub user: String,
}

pub type HyperliquidResponse = Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClearinghouseState {
    #[serde(rename = "assetPositions")]
    pub asset_positions: Vec<AssetPosition>,
    #[serde(rename = "crossMaintenanceMarginUsed")]
    pub cross_maintenance_margin_used: String,
    #[serde(rename = "crossMarginSummary")]
    pub cross_margin_summary: MarginSummary,
    #[serde(rename = "marginSummary")]
    pub margin_summary: MarginSummary,
    pub time: i64,
    pub withdrawable: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarginSummary {
    #[serde(rename = "accountValue")]
    pub account_value: String,
    #[serde(rename = "totalMarginUsed")]
    pub total_margin_used: String,
    #[serde(rename = "totalNtlPos")]
    pub total_ntl_pos: String,
    #[serde(rename = "totalRawUsd")]
    pub total_raw_usd: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetPosition {
    pub position: Position,
    #[serde(rename = "type")]
    pub position_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub coin: String,
    pub szi: String,
    #[serde(rename = "unrealizedPnl")]
    pub unrealized_pnl: String,
    #[serde(rename = "cumFunding")]
    pub cum_funding: CumFunding,
    pub leverage: Leverage,
    #[serde(rename = "liquidationPx")]
    pub liquidation_px: Option<String>,
    #[serde(rename = "entryPx")]
    pub entry_px: String,
    #[serde(rename = "marginUsed")]
    pub margin_used: String,
    #[serde(rename = "maxLeverage")]
    pub max_leverage: u32,
    #[serde(rename = "positionValue")]
    pub position_value: String,
    #[serde(rename = "returnOnEquity")]
    pub return_on_equity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CumFunding {
    #[serde(rename = "allTime")]
    pub all_time: String,
    #[serde(rename = "sinceChange")]
    pub since_change: String,
    #[serde(rename = "sinceOpen")]
    pub since_open: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Leverage {
    #[serde(rename = "rawUsd")]
    pub raw_usd: Option<String>,
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClosedPositionRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
    #[serde(rename = "aggregateByTime")]
    pub aggregate_by_time: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub user: String,
}

/// One window in the Hyperliquid `portfolio` info response (`day`, `perpAllTime`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPeriod {
    pub vlm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserFill {
    pub coin: String,
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub side: String,
    #[serde(default)]
    pub px: String,
    #[serde(default)]
    pub sz: String,
    #[serde(rename = "closedPnl", default)]
    pub closed_pnl: String,
    #[serde(rename = "startPosition", default)]
    pub start_position: String,
    #[serde(default)]
    pub time: i64,
}

impl UserInfo {
    pub fn new(evm_address: Option<String>, solana_address: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: HYPERLIQUID_HTTP_URL.to_string(),
            evm_address,
            solana_address,
        }
    }

    pub async fn get_funding_rate(self) -> Result<HyperliquidResponse> {
        let evm_address = self
            .evm_address
            .ok_or_else(|| anyhow::Error::msg("EVM address is required for funding rate query"))?;

        let funding_rate_request = FundingRateRequest {
            funding_type: "userFunding".to_string(),
            user: evm_address,
            start_time: Utc::now().timestamp_millis(), //TODO: change this to a suitable time
        };

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/info"))
            .json(&funding_rate_request)
            .send()
            .await?;

        let data: HyperliquidResponse = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!("Failed to fetch hyperliquid funding rate. Failed with error: {err:?}");
                return Err(anyhow::Error::msg(
                    "Failed to fetch hyperliquid funding rate",
                ));
            }
        };

        Ok(data)
    }

    pub async fn get_open_positions(self) -> Result<ClearinghouseState> {
        let evm_address = self.evm_address.ok_or_else(|| {
            anyhow::Error::msg("EVM address is required for open positions query")
        })?;

        let open_position_request = OpenPositionRequest {
            position_type: "clearinghouseState".to_string(),
            user: evm_address,
        };

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/info"))
            .json(&open_position_request)
            .send()
            .await?;

        log::info!("Response {response:?}");

        let data: ClearinghouseState = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!(
                    "Failed to fetch hyperliquid open positions. Failed with error: {err:?}"
                );
                return Err(anyhow::Error::msg(
                    "Failed to fetch hyperliquid open positions",
                ));
            }
        };

        Ok(data)
    }

    pub async fn get_closed_positions(self) -> Result<Vec<UserFill>> {
        let evm_address = self.evm_address.ok_or_else(|| {
            anyhow::Error::msg("EVM address is required for closed positions query")
        })?;

        let request = ClosedPositionRequest {
            request_type: "userFills".to_string(),
            user: evm_address,
            aggregate_by_time: false,
        };

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/info"))
            .json(&request)
            .send()
            .await?;

        let data: Vec<UserFill> = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!(
                    "Failed to fetch hyperliquid closed positions. Failed with error: {err:?}"
                );
                return Err(anyhow::Error::msg(
                    "Failed to fetch hyperliquid closed positions",
                ));
            }
        };

        Ok(data)
    }

    /// `POST /info` with `type: "portfolio"` — includes per-window `vlm` (notional volume).
    pub async fn get_portfolio(self) -> Result<Vec<(String, PortfolioPeriod)>> {
        let evm_address = self
            .evm_address
            .ok_or_else(|| anyhow::Error::msg("EVM address is required for portfolio query"))?;

        let request = PortfolioRequest {
            request_type: "portfolio".to_string(),
            user: evm_address,
        };

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/info"))
            .json(&request)
            .send()
            .await?;

        let data: Vec<(String, PortfolioPeriod)> = response.json().await.map_err(|err| {
            log::error!("Failed to fetch hyperliquid portfolio: {err:?}");
            anyhow::Error::msg("Failed to fetch hyperliquid portfolio")
        })?;

        Ok(data)
    }
}

/// Prefer perp all-time volume; fall back to spot+perp all-time.
pub fn portfolio_all_time_volume_usd(periods: &[(String, PortfolioPeriod)]) -> Option<f64> {
    for key in ["perpAllTime", "allTime"] {
        if let Some((_, period)) = periods.iter().find(|(k, _)| k == key) {
            if let Ok(v) = period.vlm.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod portfolio_tests {
    use super::*;

    #[test]
    fn parses_perp_all_time_vlm() {
        let periods = vec![
            (
                "day".to_string(),
                PortfolioPeriod {
                    vlm: "10.0".to_string(),
                },
            ),
            (
                "perpAllTime".to_string(),
                PortfolioPeriod {
                    vlm: "19965.109584".to_string(),
                },
            ),
        ];
        assert_eq!(
            portfolio_all_time_volume_usd(&periods),
            Some(19965.109584)
        );
    }
}
