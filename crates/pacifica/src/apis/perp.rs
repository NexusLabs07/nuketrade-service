use std::fmt;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::PacificaClient;

#[derive(Debug, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone)]
pub enum Tif {
    GTC,
    IOC,
    ALO,
    TOB,
}

impl fmt::Display for Tif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tif::GTC => "GTC",
            Tif::IOC => "IOC",
            Tif::ALO => "ALO",
            Tif::TOB => "TOB",
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMarketOrderReq {
    pub account: String,
    pub signature: String,
    pub timestamp: u64,
    pub symbol: String,
    pub amount: u64,
    pub side: Side,
    pub slippage_percent: u32,
    pub reduce_only: bool,
    pub limit_price: Option<String>,
    pub agent_wallet: Option<String>,
    pub expiry_window: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLimitOrderReq {
    pub account: String,
    pub signature: String,
    pub timestamp: u64,
    pub symbol: String,
    pub price: String,
    pub amount: u64,
    pub side: Side,
    pub tif: String,
    pub slippage_percent: u32,
    pub reduce_only: bool,
    pub limit_price: Option<String>,
    pub agent_wallet: Option<String>,
    pub expiry_window: Option<u32>,
}

pub async fn create_perp_market_order(create_order_req: CreateMarketOrderReq) -> Result<String> {
    let client = PacificaClient::new()?;
    let order_id: String = client.create_market_order(create_order_req).await?;

    Ok(order_id)
}

pub async fn create_perp_limit_order(create_order_req: CreateLimitOrderReq) -> Result<String> {
    let client = PacificaClient::new()?;
    let order_id = client.create_limit_order(create_order_req).await?;

    Ok(order_id)
}
