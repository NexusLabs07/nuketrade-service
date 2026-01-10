use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use hex::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha3::{Digest, Keccak256};

use crate::{
    HYPERLIQUID_HTTP_TESTNET_URL, HYPERLIQUID_HTTP_URL, HyperliquidMarketPrice,
    utils::{
        asset_index_converter::perp_index_to_ticker,
        signing::{create_mainnet_exchange_typed_data, create_testnet_exchange_typed_data},
    },
};

use super::market_slippage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpSlParams {
    #[serde(rename = "assetId")]
    pub asset_id: u32,
    #[serde(rename = "isLong")]
    pub is_long: bool,
    #[serde(rename = "currentPositionSize")]
    pub current_position_size: String,
    #[serde(rename = "finalTakeProfitPrice")]
    pub final_take_profit_price: Option<String>,
    #[serde(rename = "finalStopLossPrice")]
    pub final_stop_loss_price: Option<String>,
    #[serde(rename = "takeProfitSize")]
    pub take_profit_size: Option<String>,
    #[serde(rename = "stopLossSize")]
    pub stop_loss_size: Option<String>,
    #[serde(rename = "vaultAddress")]
    pub vault_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTpSlParams {
    #[serde(rename = "assetId")]
    pub asset_id: u32,
    #[serde(rename = "orderIds")]
    pub order_ids: Vec<u64>,
    #[serde(rename = "vaultAddress")]
    pub vault_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTpSlParams {
    #[serde(rename = "assetId")]
    pub asset_id: u32,
    #[serde(rename = "isLong")]
    pub is_long: bool,
    #[serde(rename = "currentPositionSize")]
    pub current_position_size: String,
    #[serde(rename = "finalTakeProfitPrice")]
    pub final_take_profit_price: Option<String>,
    #[serde(rename = "finalStopLossPrice")]
    pub final_stop_loss_price: Option<String>,
    #[serde(rename = "vaultAddress")]
    pub vault_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TpSlTrigger {
    #[serde(rename = "isMarket")]
    is_market: bool,
    #[serde(rename = "triggerPx")]
    trigger_px: String,
    tpsl: String, // "tp" or "sl"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TpSlOrderType {
    trigger: TpSlTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TpSlOrder {
    a: u32,    // asset id
    b: bool,   // is buy
    p: String, // price
    s: String, // size
    r: bool,   // reduce only
    t: TpSlOrderType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CancelItem {
    a: u32,
    o: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpSlResponse {
    pub action: Value,
    pub nonce: u64,
    #[serde(rename = "typedData")]
    pub typed_data: Value,
    pub endpoint: String,
}

pub struct TpSlManager {
    base_url: String,
    is_testnet: bool,
}

impl TpSlManager {
    pub fn new(is_testnet: bool) -> Self {
        let base_url = if is_testnet {
            HYPERLIQUID_HTTP_TESTNET_URL.to_string()
        } else {
            HYPERLIQUID_HTTP_URL.to_string()
        };

        Self {
            base_url,
            is_testnet,
        }
    }

    /// Places take-profit and/or stop-loss orders for a position
    pub async fn place_tp_sl_orders(&self, params: TpSlParams) -> Result<TpSlResponse> {
        let mut orders = Vec::new();

        let asset_name = perp_index_to_ticker(params.asset_id as usize).await?;

        let market_price_helper = HyperliquidMarketPrice::new();
        let tick_info = market_price_helper
            .get_tick_and_lot_size(&asset_name, "perps")
            .await?;

        // Take Profit Order
        if let Some(ref tp_price) = params.final_take_profit_price {
            let position_size: f64 = params.current_position_size.parse()?;

            if position_size > 0.0 {
                // Use provided TP size or default to full position size
                let tp_size = params
                    .take_profit_size
                    .as_ref()
                    .unwrap_or(&params.current_position_size);

                let tp_price_f64: f64 = tp_price.parse()?;

                // If long, decrease the price by marketSlippage percentage; otherwise increase it
                let price_with_slippage = if params.is_long {
                    tick_info.round_price(
                        tp_price_f64 - (tp_price_f64 * market_slippage / 100.0),
                        &asset_name,
                    )
                } else {
                    tick_info.round_price(
                        tp_price_f64 + (tp_price_f64 * market_slippage / 100.0),
                        &asset_name,
                    )
                };

                let tp_size_f64: f64 = tp_size.parse()?;

                orders.push(TpSlOrder {
                    a: params.asset_id,
                    b: !params.is_long, // Opposite of position direction
                    p: price_with_slippage,
                    s: tick_info.round_size(tp_size_f64),
                    r: true, // Reduce only
                    t: TpSlOrderType {
                        trigger: TpSlTrigger {
                            is_market: true,
                            trigger_px: tick_info.round_price(tp_price_f64, &asset_name),
                            tpsl: "tp".to_string(),
                        },
                    },
                });
            }
        }

        // Stop Loss Order
        if let Some(ref sl_price) = params.final_stop_loss_price {
            let position_size: f64 = params.current_position_size.parse()?;

            if position_size > 0.0 {
                // Use provided SL size or default to full position size
                let sl_size = params
                    .stop_loss_size
                    .as_ref()
                    .unwrap_or(&params.current_position_size);

                let sl_price_f64: f64 = sl_price.parse()?;

                let price_with_slippage = if params.is_long {
                    tick_info.round_price(
                        sl_price_f64 - (sl_price_f64 * market_slippage / 100.0),
                        &asset_name,
                    )
                } else {
                    tick_info.round_price(
                        sl_price_f64 + (sl_price_f64 * market_slippage / 100.0),
                        &asset_name,
                    )
                };

                let sl_size_f64: f64 = sl_size.parse()?;

                orders.push(TpSlOrder {
                    a: params.asset_id,
                    b: !params.is_long, // Opposite of position direction
                    p: price_with_slippage,
                    s: tick_info.round_size(sl_size_f64),
                    r: true, // Reduce only
                    t: TpSlOrderType {
                        trigger: TpSlTrigger {
                            is_market: true,
                            trigger_px: tick_info.round_price(sl_price_f64, &asset_name),
                            tpsl: "sl".to_string(),
                        },
                    },
                });
            }
        }

        if orders.is_empty() {
            return Err(anyhow!(
                "No take profit or stop loss price provided, or position size is zero."
            ));
        }

        let action = json!({
            "type": "order",
            "orders": orders,
            "grouping": "na",
            //TODO: add builder fees
            // "builder": {
            //     "b": HYPERLIQUID_BUILDER_ADDRESS,
            //     "f": HYPERLIQUID_PERP_BUILDER_FEE,
            // }
        });

        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        let typed_data = if self.is_testnet {
            create_testnet_exchange_typed_data(&action, nonce, params.vault_address.as_deref())?
        } else {
            create_mainnet_exchange_typed_data(&action, nonce, params.vault_address.as_deref())?
        };

        Ok(TpSlResponse {
            action,
            nonce,
            typed_data,
            endpoint: format!("{}/exchange", self.base_url),
        })
    }

    /// Cancels existing TP/SL orders by their order IDs
    pub async fn cancel_tp_sl_orders(&self, params: CancelTpSlParams) -> Result<TpSlResponse> {
        let cancels: Vec<CancelItem> = params
            .order_ids
            .iter()
            .map(|&order_id| CancelItem {
                a: params.asset_id,
                o: order_id,
            })
            .collect();

        let action = json!({
            "type": "cancel",
            "cancels": cancels,
        });

        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        // Create action hash manually (bigint issue with cancelling orders)
        let msgpack_bytes = rmp_serde::to_vec(&action)?;
        let additional_bytes_length = if params.vault_address.is_some() {
            29
        } else {
            9
        };

        let mut data = vec![0u8; msgpack_bytes.len() + additional_bytes_length];
        data[..msgpack_bytes.len()].copy_from_slice(&msgpack_bytes);

        let nonce_bytes = nonce.to_be_bytes();
        data[msgpack_bytes.len()..msgpack_bytes.len() + 8].copy_from_slice(&nonce_bytes);

        if let Some(ref vault_addr) = params.vault_address {
            data[msgpack_bytes.len() + 8] = 1;
            let vault_bytes = decode(vault_addr.trim_start_matches("0x"))?;
            data[msgpack_bytes.len() + 9..msgpack_bytes.len() + 29].copy_from_slice(&vault_bytes);
        } else {
            data[msgpack_bytes.len() + 8] = 0;
        }

        let mut hasher = Keccak256::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        let action_hash = format!("0x{}", encode(hash));

        let typed_data = json!({
            "domain": {
                "name": "Exchange",
                "version": "1",
                "chainId": 1337,
                "verifyingContract": "0x0000000000000000000000000000000000000000"
            },
            "types": {
                "Agent": [
                    { "name": "source", "type": "string" },
                    { "name": "connectionId", "type": "bytes32" }
                ]
            },
            "primaryType": "Agent",
            "message": {
                "source": "a",
                "connectionId": action_hash
            }
        });

        Ok(TpSlResponse {
            action,
            nonce,
            typed_data,
            endpoint: format!("{}/exchange", self.base_url),
        })
    }

    /// Updates existing TP/SL orders with new prices
    pub async fn update_tp_sl_orders(
        &self,
        params: UpdateTpSlParams,
        existing_tp_order_id: Option<u64>,
        existing_sl_order_id: Option<u64>,
    ) -> Result<Vec<TpSlResponse>> {
        let mut transactions = Vec::new();

        // First cancel existing orders if any IDs are provided
        let mut order_ids_to_cancel = Vec::new();
        if let Some(tp_id) = existing_tp_order_id {
            order_ids_to_cancel.push(tp_id);
        }
        if let Some(sl_id) = existing_sl_order_id {
            order_ids_to_cancel.push(sl_id);
        }

        if !order_ids_to_cancel.is_empty() {
            let cancel_response = self
                .cancel_tp_sl_orders(CancelTpSlParams {
                    asset_id: params.asset_id,
                    order_ids: order_ids_to_cancel,
                    vault_address: params.vault_address.clone(),
                })
                .await?;
            transactions.push(cancel_response);
        }

        // Place new orders (only if TP or SL price is provided)
        if params.final_take_profit_price.is_some() || params.final_stop_loss_price.is_some() {
            let place_params = TpSlParams {
                asset_id: params.asset_id,
                is_long: params.is_long,
                current_position_size: params.current_position_size,
                final_take_profit_price: params.final_take_profit_price,
                final_stop_loss_price: params.final_stop_loss_price,
                take_profit_size: None,
                stop_loss_size: None,
                vault_address: params.vault_address,
            };

            let place_response = self.place_tp_sl_orders(place_params).await?;
            transactions.push(place_response);
        }

        Ok(transactions)
    }
}
