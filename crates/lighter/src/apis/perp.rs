use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    LIGHTER_HTTP_URL,
    client::{L2CreateOrderTxInfo, LighterClient},
    constants::MAINNET_CHAIN_ID,
};

/// Create Order Transaction Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderTxReq {
    pub market_index: u8,
    pub client_order_index: i64,
    pub base_amount: i64,
    pub price: i32,
    pub is_ask: u8,
    pub order_type: u8,
    pub time_in_force: u8,
    pub reduce_only: bool,
    pub trigger_price: u32,
    pub order_expiry: i64,
    pub account_index: i64,
    pub api_key_index: u8,
}

pub async fn create_perp_order(create_order_req: CreateOrderTxReq) -> Result<L2CreateOrderTxInfo> {
    let CreateOrderTxReq {
        market_index,
        client_order_index,
        base_amount,
        price,
        is_ask,
        reduce_only,
        account_index,
        api_key_index,
        ..
    } = create_order_req;

    let api_client_url = LIGHTER_HTTP_URL;

    let tx_client = LighterClient::new(
        &api_client_url,
        account_index,
        api_key_index,
        MAINNET_CHAIN_ID,
    )?;

    let result: L2CreateOrderTxInfo = tx_client
        .create_market_order(
            market_index,
            client_order_index,
            base_amount,
            price,
            is_ask,
            reduce_only,
            None,
        )
        .await?;

    Ok(result)
}
