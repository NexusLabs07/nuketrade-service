use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::LIGHTER_HTTP_URL;

/// Top-level response of `GET /api/v1/accountsByL1Address?l1_address=<evm>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountsByL1AddressResponse {
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub l1_address: String,
    #[serde(default)]
    pub sub_accounts: Vec<LighterAccountSummary>,
}

/// Lightweight per-sub-account summary from `accountsByL1Address`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LighterAccountSummary {
    #[serde(default)]
    pub index: u64,
    #[serde(default)]
    pub account_type: i32,
    #[serde(default)]
    pub l1_address: String,
    #[serde(default)]
    pub available_balance: String,
    #[serde(default)]
    pub collateral: String,
    #[serde(default)]
    pub status: i32,
}

/// Detailed response from `GET /api/v1/account?by=l1_address&value=<evm>`,
/// which includes per-position realised + unrealised PnL.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountByL1Response {
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub accounts: Vec<LighterAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LighterAccount {
    #[serde(default)]
    pub index: u64,
    #[serde(default)]
    pub account_type: i32,
    #[serde(default)]
    pub l1_address: String,
    #[serde(default)]
    pub available_balance: String,
    #[serde(default)]
    pub collateral: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub positions: Vec<LighterPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LighterPosition {
    #[serde(default)]
    pub market_id: u64,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub sign: i8,
    #[serde(default)]
    pub position: String,
    #[serde(default)]
    pub avg_entry_price: String,
    #[serde(default)]
    pub position_value: String,
    #[serde(default)]
    pub unrealized_pnl: String,
    #[serde(default)]
    pub realized_pnl: String,
    #[serde(default)]
    pub liquidation_price: String,
    #[serde(default)]
    pub allocated_margin: String,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub client: Client,
    pub base_url: String,
    pub evm_address: String,
}

impl UserInfo {
    pub fn new(evm_address: String) -> Self {
        Self {
            client: Client::new(),
            base_url: LIGHTER_HTTP_URL.to_string(),
            evm_address,
        }
    }

    /// Returns the lightweight sub-account summary keyed by the user's EVM (L1) address.
    pub async fn get_accounts_by_l1(&self) -> Result<AccountsByL1AddressResponse> {
        let url = format!(
            "{}/api/v1/accountsByL1Address?l1_address={}",
            self.base_url, self.evm_address
        );
        let response = self.client.get(url).send().await?;
        let data: AccountsByL1AddressResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse lighter accountsByL1Address: {e}"))?;
        Ok(data)
    }

    /// Returns the detailed account view (including positions) for the given EVM address.
    pub async fn get_account_detail(&self) -> Result<AccountByL1Response> {
        let url = format!(
            "{}/api/v1/account?by=l1_address&value={}",
            self.base_url, self.evm_address
        );
        let response = self.client.get(url).send().await?;
        let data: AccountByL1Response = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse lighter account: {e}"))?;
        Ok(data)
    }
}
