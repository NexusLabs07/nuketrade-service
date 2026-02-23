use anyhow::Result;
use chrono::Utc;
use reqwest::Client;

use crate::{
    HYPERLIQUID_HTTP_URL,
    ops::types::{FundingRateRequest, HyperliquidFundingRate, HyperliquidResponse},
};

impl HyperliquidFundingRate {
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
}
