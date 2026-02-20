use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::PACIFICA_HTTP_URL;

#[derive(Serialize, Deserialize)]
pub struct WithdrawRequest {
    pub account: String,
    pub signature: String,
    pub timestamp: u128,
    pub amount: String,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawResponse {
    pub success: bool,
}

pub async fn withdraw(payload: WithdrawRequest) -> Result<bool> {
    let client = Client::new();

    let response = client
        .post(format!("{}/account/withdraw", PACIFICA_HTTP_URL))
        .json(&payload)
        .send()
        .await?;

    let data: WithdrawResponse = match response.json().await {
        Ok(d) => d,
        Err(err) => {
            log::error!("Failed to submit pacifica withdrawal transaction {err:?}");

            return Err(anyhow::Error::msg(
                "Pacifica: Failed to get submit withdrawal transaction",
            ));
        }
    };

    Ok(data.success)
}
