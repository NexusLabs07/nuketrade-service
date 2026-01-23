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
    pub funding: String,
    pub isolated: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

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

    pub async fn get_open_positions(self) -> Result<UserPositionsResponse> {
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
            Err(err) => {
                return Err(anyhow::Error::msg("Failed to get pacifica open positions"));
            }
        };

        Ok(data)
    }
}
