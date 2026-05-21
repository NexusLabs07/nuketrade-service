use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::PHOENIX_HTTP_URL;

#[derive(Debug, Clone, Serialize, Deserialize, validator::Validate)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrderIxRequest {
    pub authority: String,
    pub side: String,
    pub symbol: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_base_lots: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price_in_ticks: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_reduce_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_payer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_cross_and_isolated_for_asset: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixInstruction {
    pub data: Vec<u8>,
    pub keys: Vec<PhoenixAccountMeta>,
    pub program_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

pub async fn build_market_order_ix(
    payload: MarketOrderIxRequest,
) -> anyhow::Result<Vec<PhoenixInstruction>> {
    let client = Client::new();

    let response = client
        .post(format!(
            "{}/v1/ix/place-isolated-market-order",
            PHOENIX_HTTP_URL
        ))
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Phoenix market order ix failed with HTTP {status}: {body}");
    }

    Ok(response.json::<Vec<PhoenixInstruction>>().await?)
}
