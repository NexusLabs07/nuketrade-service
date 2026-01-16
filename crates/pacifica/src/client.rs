use serde::{self, Deserialize};
use std::time::Duration;

use anyhow::Result;
use reqwest::{Client, StatusCode};

use crate::{
    PACIFICA_HTTP_URL,
    apis::perp::{CreateLimitOrderReq, CreateMarketOrderReq},
};

#[derive(Debug, Deserialize)]
pub struct CreateOrderResponse {
    order_id: String,
}

pub struct PacificaClient {
    pub client: Client,
    pub base_url: String,
}

impl PacificaClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            base_url: PACIFICA_HTTP_URL.to_string(),
        })
    }

    pub async fn create_market_order(
        &self,
        create_order_req: CreateMarketOrderReq,
    ) -> Result<String> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, "orders/create_market"))
            .json(&create_order_req)
            .send()
            .await?;

        let data: CreateOrderResponse = {
            if response.status().is_success() {
                let d: CreateOrderResponse = match response.json().await {
                    Ok(d) => d,
                    Err(err) => {
                        return Err(anyhow::Error::msg("Failed to create a market order"));
                    }
                };
                d
            } else {
                if response.status() == StatusCode::BAD_REQUEST {
                    return Err(anyhow::Error::msg(
                        "Failed to create a market order. Bad request",
                    ));
                } else {
                    return Err(anyhow::Error::msg("Internal server error"));
                }
            }
        };

        Ok(data.order_id)
    }

    pub async fn create_limit_order(
        &self,
        create_order_req: CreateLimitOrderReq,
    ) -> Result<String> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, "orders/create"))
            .json(&create_order_req)
            .send()
            .await?;

        let data: CreateOrderResponse = {
            if response.status().is_success() {
                let d: CreateOrderResponse = match response.json().await {
                    Ok(d) => d,
                    Err(err) => {
                        return Err(anyhow::Error::msg("Failed to create a market order"));
                    }
                };
                d
            } else {
                if response.status() == StatusCode::BAD_REQUEST {
                    return Err(anyhow::Error::msg(
                        "Failed to create a market order. Bad request",
                    ));
                } else {
                    return Err(anyhow::Error::msg("Internal server error"));
                }
            }
        };

        Ok(data.order_id)
    }
}
