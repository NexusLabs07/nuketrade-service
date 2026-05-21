use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::PHOENIX_HTTP_URL;

#[derive(Debug, Clone)]
pub struct UserInfo {
    client: Client,
    http_url: String,
    authority: String,
    pda_index: Option<u32>,
}

impl UserInfo {
    pub fn new(authority: String) -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            authority,
            pda_index: None,
        }
    }

    pub fn with_pda_index(authority: String, pda_index: u32) -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            authority,
            pda_index: Some(pda_index),
        }
    }

    pub fn with_url(authority: String, http_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            authority,
            pda_index: None,
        }
    }

    fn with_pda_query(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.pda_index {
            Some(pda_index) => request.query(&[("pdaIndex", pda_index)]),
            None => request,
        }
    }

    pub async fn get_trader_state(&self) -> Result<TraderStateResponse> {
        let request = self
            .client
            .get(format!("{}/trader/{}/state", self.http_url, self.authority));

        let response = self.with_pda_query(request).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Phoenix trader state returned HTTP {}", response.status());
        }

        Ok(response.json::<TraderStateResponse>().await?)
    }

    pub async fn get_trade_history(&self) -> Result<TradeHistoryResponse> {
        let request = self.client.get(format!(
            "{}/trader/{}/trades-history",
            self.http_url, self.authority
        ));

        let response = self.with_pda_query(request).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Phoenix trade history returned HTTP {}", response.status());
        }

        Ok(response.json::<TradeHistoryResponse>().await?)
    }

    pub async fn get_funding_history(&self) -> Result<FundingHistoryResponse> {
        let request = self.client.get(format!(
            "{}/trader/{}/funding-history",
            self.http_url, self.authority
        ));

        let response = self.with_pda_query(request).send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Phoenix funding history returned HTTP {}",
                response.status()
            );
        }

        Ok(response.json::<FundingHistoryResponse>().await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderStateResponse {
    pub authority: String,
    pub pda_index: u32,
    pub slot: i64,
    pub slot_index: i64,
    #[serde(default)]
    pub traders: Vec<PhoenixTrader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixTrader {
    pub authority: String,
    pub trader_key: String,
    pub trader_pda_index: u32,
    pub trader_subaccount_index: u32,

    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub risk_state: String,
    #[serde(default)]
    pub risk_tier: String,

    #[serde(default)]
    pub collateral_balance: String,
    #[serde(default)]
    pub effective_collateral: String,
    #[serde(default)]
    pub effective_collateral_for_withdrawals: String,
    #[serde(default)]
    pub portfolio_value: String,
    #[serde(default)]
    pub initial_margin: String,
    #[serde(default)]
    pub maintenance_margin: String,
    #[serde(default)]
    pub unrealized_pnl: String,
    #[serde(default)]
    pub unsettled_funding_owed: String,
    #[serde(default)]
    pub accumulated_funding: String,

    #[serde(default)]
    pub positions: Vec<PhoenixPosition>,

    #[serde(default)]
    pub limit_orders: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixPosition {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub position_size: String,
    #[serde(default)]
    pub entry_price: String,
    #[serde(default)]
    pub liquidation_price: String,
    #[serde(default)]
    pub position_initial_margin: String,
    #[serde(default)]
    pub initial_margin: String,
    #[serde(default)]
    pub maintenance_margin: String,
    #[serde(default)]
    pub position_value: String,
    #[serde(default)]
    pub unrealized_pnl: String,
    #[serde(default)]
    pub unsettled_funding: String,
    #[serde(default)]
    pub accumulated_funding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryResponse {
    #[serde(default)]
    pub data: Vec<PhoenixTrade>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixTrade {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub market_symbol: String,
    #[serde(default)]
    pub price: String,
    #[serde(default)]
    pub realized_pnl: String,
    #[serde(default)]
    pub base_lots_delta: String,
    #[serde(default)]
    pub virtual_quote_lots_delta: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingHistoryResponse {
    #[serde(default)]
    pub events: Vec<PhoenixFundingEvent>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixFundingEvent {
    #[serde(default)]
    pub funding_payment: String,
    #[serde(default)]
    pub funding_rate_percentage: String,
    #[serde(default)]
    pub position_side: String,
    #[serde(default)]
    pub position_size: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timestamp: String,
}
