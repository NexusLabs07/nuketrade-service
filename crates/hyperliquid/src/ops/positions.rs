use anyhow::Result;
use reqwest::Client;

use crate::{
    HYPERLIQUID_HTTP_URL,
    ops::types::{
        ClearinghouseState, ClosedPositionRequest, HyperliquidPositions, OpenPositionRequest,
        UserFill,
    },
};

impl HyperliquidPositions {
    pub fn new(evm_address: Option<String>, solana_address: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: HYPERLIQUID_HTTP_URL.to_string(),
            evm_address,
            solana_address,
        }
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
}
