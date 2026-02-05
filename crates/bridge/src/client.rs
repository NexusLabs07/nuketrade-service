use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::RELAY_API_URL;

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub user: String,
    #[serde(rename = "originChainId")]
    pub origin_chain_id: u64,
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
    #[serde(rename = "originCurrency")]
    pub origin_currency: String,
    #[serde(rename = "destinationCurrency")]
    pub destination_currency: String,
    pub amount: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "usePermit")]
    pub use_permit: bool,
    pub recipient: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermitRequest {
    pub signature: String,
    pub kind: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub api: String,
}

pub type RelayResponse = Value;

pub struct BridgeClient {
    pub client: Client,
    pub base_url: String,
}

impl BridgeClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: RELAY_API_URL.to_string(),
        }
    }

    pub async fn quote(&self, quote_request: QuoteRequest) -> Result<RelayResponse> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/quote/v2"))
            .json(&quote_request)
            .send()
            .await?;

        let data: RelayResponse = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!(
                    "Failed to fetch quote using relay. Failed with error: {:?}",
                    err
                );

                return Err(anyhow::Error::msg("Failed to submit permit using relay"));
            }
        };

        Ok(data)
    }

    pub async fn execute_permit(&self, permit_request: PermitRequest) -> Result<RelayResponse> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/execute/permits"))
            .query(&permit_request)
            .send()
            .await?;

        let data: RelayResponse = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!("Failed to submit permit. Failed with error: {:?}", err);

                return Err(anyhow::Error::msg("Failed to submit permit using relay"));
            }
        };

        Ok(data)
    }
}
