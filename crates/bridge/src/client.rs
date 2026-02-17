use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::RELAY_API_URL;

const PROTOCOL_FEES: u64 = 200_000;

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
    pub api: Option<String>,
}

#[derive(Debug, Serialize)]
struct PermitRequestBody {
    pub kind: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub steps: Value,
    #[serde(rename = "timeEstimate")]
    pub time_estimate: u64,
    #[serde(rename = "amountIn")]
    pub amount_in: String,
    #[serde(rename = "amountOut")]
    pub amount_out: String,
    #[serde(rename = "minimumReceived")]
    pub minimum_received: String,
    pub rate: String,
    #[serde(rename = "gasFeeUsd")]
    pub gas_fee_usd: String,
    #[serde(rename = "relayFeeUsd")]
    pub relay_fee_usd: String,
    #[serde(rename = "protocolFees")]
    pub protocol_fees: String,
}

fn parse_quote(data: &Value) -> Result<QuoteResponse> {
    let details = &data["details"];
    let fees = &data["fees"];

    let amount_out = (details["currencyOut"]["amountFormatted"]
        .as_str()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0)
        - (PROTOCOL_FEES as f64 / 1_000_000.0)) // Convert from micro units to standard units
        .to_string();

    let minimum_received = (((details["currencyOut"]["minimumAmount"]
        .as_str()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0))
        - PROTOCOL_FEES as f64)
        / 1_000_000.0)
        .to_string();

    let protocol_fees = (PROTOCOL_FEES as f64 / 1_000_000.0).to_string(); // Convert from micro units to standard units

    Ok(QuoteResponse {
        steps: data["steps"].clone(),
        time_estimate: details["timeEstimate"].as_u64().unwrap_or(0),
        amount_in: details["currencyIn"]["amountFormatted"]
            .as_str()
            .unwrap_or("0")
            .to_string(),
        amount_out,
        minimum_received,
        rate: details["rate"].as_str().unwrap_or("0").to_string(),
        gas_fee_usd: fees["relayerGas"]["amountUsd"]
            .as_str()
            .unwrap_or("0")
            .to_string(),
        relay_fee_usd: fees["relayer"]["amountUsd"]
            .as_str()
            .unwrap_or("0")
            .to_string(),
        protocol_fees,
    })
}

pub struct BridgeClient {
    pub client: Client,
    pub base_url: String,
    pub api_key: String,
}

impl BridgeClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: RELAY_API_URL.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn quote(&self, quote_request: QuoteRequest) -> Result<QuoteResponse> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/quote/v2"))
            .header("x-api-key", &self.api_key)
            .json(&quote_request)
            .send()
            .await?;

        let data: Value = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!("Failed to fetch quote using relay. Failed with error: {err:?}");

                return Err(anyhow::Error::msg("Failed to fetch quote using relay"));
            }
        };

        parse_quote(&data)
    }

    pub async fn execute_permit(&self, permit_request: PermitRequest) -> Result<Value> {
        let body = PermitRequestBody {
            kind: permit_request.kind,
            request_id: permit_request.request_id,
            api: permit_request.api,
        };

        let response = self
            .client
            .post(format!("{}{}", self.base_url, "/execute/permits"))
            .query(&[("signature", &permit_request.signature)])
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let data: Value = match response.json().await {
            Ok(d) => d,
            Err(err) => {
                log::error!("Failed to submit permit. Failed with error: {err:?}");

                return Err(anyhow::Error::msg("Failed to submit permit using relay"));
            }
        };

        Ok(data)
    }
}
