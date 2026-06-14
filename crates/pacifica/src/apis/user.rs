use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::PACIFICA_HTTP_URL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPositionsResponse {
    pub success: bool,
    pub data: Option<Vec<UserPosition>>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub last_order_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPosition {
    pub symbol: String,
    pub side: String,
    pub amount: String,
    pub entry_price: String,
    #[serde(default)]
    pub margin: Option<String>,
    #[serde(default)]
    pub liquidation_price: Option<String>,
    #[serde(default)]
    pub funding: Option<String>,
    pub isolated: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSetting {
    pub symbol: String,
    pub isolated: bool,
    pub leverage: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Body of `data` for `GET /account/settings` (current Pacifica API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSettingsData {
    #[serde(default)]
    pub auto_lend_disabled: Option<bool>,
    #[serde(default)]
    pub margin_settings: Vec<AccountSetting>,
    #[serde(default)]
    pub spot_settings: Vec<serde_json::Value>,
}

/// Pacifica has returned `data` as either a bare list or a structured object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountSettingsPayload {
    LegacyMarginSettings(Vec<AccountSetting>),
    Structured(AccountSettingsData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSettingsResponse {
    pub success: bool,
    pub data: Option<AccountSettingsPayload>,
    pub error: Option<String>,
    pub code: Option<String>,
}

impl AccountSettingsResponse {
    /// Per-symbol margin mode + leverage rows (empty slice if structured but no rows).
    pub fn margin_settings(&self) -> Option<&[AccountSetting]> {
        self.data.as_ref().map(|p| match p {
            AccountSettingsPayload::LegacyMarginSettings(rows) => rows.as_slice(),
            AccountSettingsPayload::Structured(s) => s.margin_settings.as_slice(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPositionsHistoryResponse {
    pub success: bool,
    pub data: Option<Vec<UserPositionHistory>>,
    pub error: Option<String>,
    pub code: Option<String>,
    pub last_order_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPositionHistory {
    #[serde(default)]
    pub order_id: u64,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub side: String,
    #[serde(default)]
    pub order_type: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub is_liquidation: bool,
    #[serde(default)]
    pub execution_price: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfoResponse {
    pub success: bool,
    pub data: Option<AccountInfo>,
    pub error: Option<String>,
    pub code: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountInfo {
    #[serde(default)]
    pub balance: String,
    #[serde(default)]
    pub account_equity: String,
    #[serde(default)]
    pub available_to_spend: String,
    #[serde(default)]
    pub available_to_withdraw: String,
    #[serde(default)]
    pub total_margin_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeHistoryResponse {
    pub success: bool,
    pub data: Option<Vec<TradeHistoryEntry>>,
    pub error: Option<String>,
    pub code: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeHistoryEntry {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub side: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub entry_price: String,
    #[serde(default)]
    pub pnl: String,
    #[serde(default)]
    pub fee: String,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub client: Client,
    pub base_url: String,
    pub solana_address: String,
}

impl UserInfo {
    pub fn new(solana_address: String) -> Self {
        Self {
            client: Client::new(),
            base_url: PACIFICA_HTTP_URL.to_string(),
            solana_address,
        }
    }

    pub async fn get_open_positions(&self) -> Result<UserPositionsResponse> {
        let response = self
            .client
            .get(format!(
                "{}{}{}",
                self.base_url, "/positions?account=", self.solana_address
            ))
            .send()
            .await?;

        let data: UserPositionsResponse = match response.json().await {
            Ok(d) => d,
            Err(_err) => {
                return Err(anyhow::Error::msg("Failed to get pacifica open positions"));
            }
        };

        Ok(data)
    }

    pub async fn get_account_settings(&self) -> Result<AccountSettingsResponse> {
        let response = self
            .client
            .get(format!(
                "{}{}{}",
                self.base_url, "/account/settings?account=", self.solana_address
            ))
            .send()
            .await?;

        let data: AccountSettingsResponse = match response.json().await {
            Ok(d) => d,
            Err(_err) => {
                return Err(anyhow::Error::msg(
                    "Failed to get pacifica user account setting",
                ));
            }
        };

        Ok(data)
    }

    pub async fn get_closed_positions(&self) -> Result<UserPositionsHistoryResponse> {
        let response = self
            .client
            .get(format!(
                "{}{}{}",
                self.base_url, "/positions/history?account=", self.solana_address
            ))
            .send()
            .await?;

        let data: UserPositionsHistoryResponse = match response.json().await {
            Ok(d) => d,
            Err(_err) => {
                return Err(anyhow::Error::msg(
                    "Failed to get pacifica closed positions",
                ));
            }
        };

        Ok(data)
    }

    pub async fn get_account_info(&self) -> Result<AccountInfoResponse> {
        let response = self
            .client
            .get(format!(
                "{}/account?account={}",
                self.base_url, self.solana_address
            ))
            .send()
            .await?;

        let data: AccountInfoResponse = match response.json().await {
            Ok(d) => d,
            Err(_err) => {
                return Err(anyhow::Error::msg("Failed to get pacifica account info"));
            }
        };

        Ok(data)
    }

    pub async fn get_trade_history(&self) -> Result<TradeHistoryResponse> {
        let response = self
            .client
            .get(format!(
                "{}/trades/history?account={}&limit=1000",
                self.base_url, self.solana_address
            ))
            .send()
            .await?;

        let data: TradeHistoryResponse = match response.json().await {
            Ok(d) => d,
            Err(_err) => {
                return Err(anyhow::Error::msg("Failed to get pacifica trade history"));
            }
        };

        Ok(data)
    }

    /// `GET /portfolio/volume` — per-account notional volume by window.
    pub async fn get_portfolio_volume(&self) -> Result<PortfolioVolumeResponse> {
        let response = self
            .client
            .get(format!(
                "{}/portfolio/volume?account={}",
                self.base_url, self.solana_address
            ))
            .send()
            .await?;

        let data: PortfolioVolumeResponse = response
            .json()
            .await
            .map_err(|_| anyhow::Error::msg("Failed to get pacifica portfolio volume"))?;

        Ok(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioVolumeData {
    pub volume_1d: String,
    pub volume_7d: String,
    pub volume_14d: String,
    pub volume_30d: String,
    pub volume_all_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioVolumeResponse {
    pub success: bool,
    pub data: Option<PortfolioVolumeData>,
    pub error: Option<String>,
    pub code: Option<String>,
}

impl PortfolioVolumeData {
    pub fn volume_all_time_usd(&self) -> Option<f64> {
        self.volume_all_time.parse().ok()
    }
}
