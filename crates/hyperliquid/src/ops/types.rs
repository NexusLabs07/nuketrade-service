use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct HyperliquidPositions {
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

// --------------------Funding rate

pub struct HyperliquidFundingRate {
    pub client: Client,
    pub base_url: String,
    pub evm_address: Option<String>,
    pub solana_address: Option<String>,
}
