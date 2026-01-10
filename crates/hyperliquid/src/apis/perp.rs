use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    HyperliquidMarketPrice, MarketPrice, TickAndLotSize, apis::market_slippage,
    perp_ticker_to_index, spot_ticker_to_index, utils::signing::create_mainnet_exchange_typed_data,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpOrderRequest {
    pub asset_index: u32,
    pub asset_name: String,
    pub price: Option<f64>,
    pub size: String,
    pub is_market: bool,
    pub vault_address: Option<String>,
    pub is_long: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    pub asset_ticker: String,
    pub order_id: String,
    pub is_perp: bool,
    pub vault_address: Option<String>,
}

pub async fn create_perp_position_typed_data(order_request: PerpOrderRequest) -> Result<Value> {
    let PerpOrderRequest {
        price,
        size,
        asset_index,
        asset_name,
        is_long,
        is_market,
        vault_address,
    } = order_request;

    if !is_market && price.is_none() {
        //TODO: Return an error back here
    }

    let market_helper = HyperliquidMarketPrice::new();

    let tick_info: TickAndLotSize = market_helper
        .get_tick_and_lot_size(&asset_name, "perps")
        .await?;

    let mut buying_price = {
        let side = if is_long == true { "buy" } else { "sell" };

        if is_market == true {
            let info: MarketPrice = market_helper
                .get_market_price_for_trading(&asset_name, "perps", side)
                .await?;
            info.price
        } else {
            if price.is_none() {
                return Err(anyhow::Error::msg("Price not provided"));
            }

            price.unwrap()
        }
    };

    if price.unwrap() < 0.0 {
        return Err(anyhow::Error::msg("Price cannot be less than 0"));
    }

    let buying_amount = size.clone();

    //TODO: Use of this?
    let float_size = match size.parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(anyhow::Error::msg("Price cannot be less than 0"));
        }
    };

    if is_market {
        if is_long {
            buying_price = buying_price + buying_price * (market_slippage / 100.0);
        } else {
            buying_price = buying_price - buying_price * (market_slippage / 100.0);
        }
    }

    let tif = if is_market { "Ioc" } else { "Gtc" };

    let action = serde_json::json!({
        "type": "order",
        "orders": [
            {
                "a": asset_index,
                "b": is_long,
                "p": tick_info.round_price(buying_price, &asset_name),
                "s": buying_amount,
                "r": false,
                "t": { "limit": { "tif": tif}}
            }
        ],
        "grouping": "na",
        //TODO: Add builder code later
    });

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let typed_data = create_mainnet_exchange_typed_data(&action, nonce, vault_address.as_deref())?;

    Ok(serde_json::json!({
        "typedData": typed_data,
        "action": action,
        "nonce": nonce,
        "endpoint": format!("{}/{}", market_helper.base_url, "exchange")
    }))
}

pub async fn close_perp_position_typed_data(order_request: PerpOrderRequest) -> Result<Value> {
    let PerpOrderRequest {
        asset_index,
        asset_name,
        price,
        size,
        vault_address,
        is_long,
        is_market,
    } = order_request;

    let market_helper = HyperliquidMarketPrice::new();

    let tick_info: TickAndLotSize = market_helper
        .get_tick_and_lot_size(&asset_name, "perps")
        .await?;

    // Since we are closing the position, the order should be reversed
    // if isLong then sell else buy
    let side = if is_long { "sell" } else { "buy" };

    let mut selling_price = if is_market {
        let info: MarketPrice = market_helper
            .get_market_price_for_trading(&asset_name, "perps", side)
            .await?;
        info.price
    } else {
        price.ok_or_else(|| anyhow::Error::msg("Price not provided for limit order"))?
    };

    // If the order is market order, then add the slippage
    if is_market {
        if is_long {
            // If position is long then we are opening a short so subtract the slippage
            selling_price = selling_price - selling_price * (market_slippage / 100.0);
        } else {
            // If position is short then we are opening a long so add the slippage
            selling_price = selling_price + selling_price * (market_slippage / 100.0);
        }
    }

    let tif = if is_market { "Ioc" } else { "Gtc" };

    let action = serde_json::json!({
        "type": "order",
        "orders": [
            {
                "a": asset_index,
                "b": !is_long,  // Opposite of position direction to close
                "p": tick_info.round_price(selling_price, &asset_name),
                "s": size,
                "r": true,  // reduce-only flag set to true
                "t": { "limit": { "tif": tif } }
            }
        ],
        "grouping": "na",
        //TODO: add builder later
        // "builder": {
        //     "b": "0x88242eab04e1b2d4234e5e09d87c36331e5eb4c9",
        //     "f": "5"
        // }
    });

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let typed_data = create_mainnet_exchange_typed_data(&action, nonce, vault_address.as_deref())?;

    Ok(serde_json::json!({
        "action": action,
        "typedData": typed_data,
        "nonce": nonce,
        "endpoint": format!("{}/{}", market_helper.base_url, "exchange")
    }))
}

pub async fn close_all_perp_position_typed_data(
    orders_request: Vec<PerpOrderRequest>,
) -> Result<Vec<Value>> {
    let mut close_position_typed_data_array = Vec::new();

    for order_request in orders_request {
        let typed_data = close_perp_position_typed_data(order_request).await?;
        close_position_typed_data_array.push(typed_data);
    }

    Ok(close_position_typed_data_array)
}

pub async fn cancel_order_typed_data(request: CancelOrderRequest) -> Result<Value> {
    let CancelOrderRequest {
        asset_ticker,
        order_id,
        is_perp,
        vault_address,
    } = request;

    //TODO: throw error if spot
    // Get asset index based on market type
    let index = if !is_perp {
        // Spot asset
        let spot_index = spot_ticker_to_index(&asset_ticker, None)
            .await
            .map_err(|_| {
                anyhow::Error::msg(format!(
                    "Could not find asset index for ticker: {}",
                    asset_ticker
                ))
            })?;
        spot_index as i64
    } else {
        // Perp asset
        let perp_index = perp_ticker_to_index(&asset_ticker).await.map_err(|_| {
            anyhow::Error::msg(format!(
                "Could not find asset index for ticker: {}",
                asset_ticker
            ))
        })?;
        perp_index as i64
    };

    // Parse order_id to u64
    let order_id_num: u64 = order_id
        .parse()
        .map_err(|_| anyhow::Error::msg("Invalid order_id format"))?;

    // Create cancel action
    let action = serde_json::json!({
        "type": "cancel",
        "cancels": [
            {
                "a": index,
                "o": order_id_num,
            }
        ],
    });

    // Get current timestamp in milliseconds
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Create typed data using the utility function
    let typed_data = create_mainnet_exchange_typed_data(&action, nonce, vault_address.as_deref())?;

    let market_helper = HyperliquidMarketPrice::new();

    Ok(serde_json::json!({
        "typedData": typed_data,
        "action": action,
        "nonce": nonce,
        "endpoint": format!("{}/{}", market_helper.base_url, "exchange")
    }))
}
