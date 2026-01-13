use std::time::Duration;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::{
    ORDER_TYPE_MARKET,
    constants::{
        MAX_ACCOUNT_INDEX, MAX_API_KEY_INDEX, MAX_MARKET_INDEX, MIN_ACCOUNT_INDEX, MIN_NONCE,
        MIN_ORDER_PRICE, TIME_IN_FORCE_IMMEDIATE_OR_CANCEL, TX_TYPE_L2_CREATE_ORDER,
    },
};

#[derive(Debug)]
pub struct HttpClient {
    client: Client,
    endpoint: String,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: &str) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            client,
            endpoint: base_url.to_string(),
        })
    }

    /// Get the next nonce for an account and API key
    pub async fn get_next_nonce(&self, account_index: i64, api_key_index: u8) -> Result<i64> {
        let url = format!(
            "{}/api/v1/nextNonce?account_index={}&api_key_index={}",
            self.endpoint, account_index, api_key_index
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::Error::msg(format!(
                "Failed to get next nonce: {}",
                response.status()
            )));
        }

        #[derive(Deserialize)]
        struct NonceResponse {
            nonce: i64,
        }

        let nonce_response: NonceResponse = response.json().await?;
        Ok(nonce_response.nonce)
    }
}

/// Transaction options for customizing transaction parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactOpts {
    pub from_account_index: Option<i64>,
    pub api_key_index: Option<u8>,
    #[serde(default)]
    pub expired_at: i64,
    pub nonce: Option<i64>,
    #[serde(default)]
    pub dry_run: bool,
}

impl TransactOpts {}

//--------------------------------------//

#[derive(Debug, Serialize, Deserialize)]
pub struct SendTxRequest {
    pub tx_type: u8,
    pub tx_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderTxReq {
    pub market_index: u8,
    pub client_order_index: i64,
    pub base_amount: i64,
    pub price: u32,
    pub is_ask: u8,
    pub order_type: u8,
    pub time_in_force: u8,
    pub reduce_only: u8,
    pub trigger_price: u32,
    pub order_expiry: i64,
}

/// Order information structure used in order-related transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    pub market_index: u8,
    pub client_order_index: i64,
    pub base_amount: i64,
    pub price: u32,
    pub is_ask: u8,
    pub order_type: u8,
    pub time_in_force: u8,
    pub reduce_only: u8,
    pub trigger_price: u32,
    pub order_expiry: i64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CreateOrderTxInfo {
    #[serde(rename = "AccountIndex")]
    pub account_index: i64,
    #[serde(rename = "ApiKeyIndex")]
    pub api_key_index: u8,
    // Flatten order_info fields to top level with PascalCase
    #[serde(rename = "MarketIndex")]
    pub market_index: u8,
    #[serde(rename = "ClientOrderIndex")]
    pub client_order_index: i64,
    #[serde(rename = "BaseAmount")]
    pub base_amount: i64,
    #[serde(rename = "Price")]
    pub price: u32,
    #[serde(rename = "IsAsk")]
    pub is_ask: u8,
    #[serde(rename = "Type")]
    pub order_type: u8,
    #[serde(rename = "TimeInForce")]
    pub time_in_force: u8,
    #[serde(rename = "ReduceOnly")]
    pub reduce_only: u8,
    #[serde(rename = "TriggerPrice")]
    pub trigger_price: u32,
    #[serde(rename = "OrderExpiry")]
    pub order_expiry: i64,
    #[serde(rename = "ExpiredAt")]
    pub expired_at: i64,
    #[serde(rename = "Nonce")]
    pub nonce: i64,
    #[serde(rename = "Sig")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Base64>")]
    #[serde(default)]
    pub sig: Option<Vec<u8>>,
    #[serde(skip)]
    pub signed_hash: Option<String>,

    // Keep original order_info for internal use (not serialized)
    #[serde(skip)]
    #[serde(default = "default_order_info")]
    pub order_info: OrderInfo,
}

fn default_order_info() -> OrderInfo {
    OrderInfo {
        market_index: 0,
        client_order_index: 0,
        base_amount: 0,
        price: 0,
        is_ask: 0,
        order_type: 0,
        time_in_force: 0,
        reduce_only: 0,
        trigger_price: 0,
        order_expiry: 0,
    }
}

/// Trait that all transaction types must implement
pub trait TxInfo {
    /// Get the transaction type identifier
    fn get_tx_type(&self) -> u8;

    /// Get transaction info as JSON string
    fn get_tx_info(&self) -> Result<String>;

    /// Get the transaction hash (if signed)
    fn get_tx_hash(&self) -> Option<String>;

    /// Validate the transaction
    fn validate(&self) -> Result<()>;

    /// Hash the transaction for signing
    fn hash(&self, lighter_chain_id: u32) -> Result<Vec<u8>>;
}

impl L2CreateOrderTxInfo {
    fn validate_order_info(&self) -> Result<()> {
        let order = &self.order_info;

        // Market index
        if order.market_index > MAX_MARKET_INDEX {
            return Err(anyhow::Error::msg(format!(
                "Lighter Market index too high: {}",
                order.market_index
            )));
        }

        // Price
        if order.price < MIN_ORDER_PRICE {
            return Err(anyhow::Error::msg(format!(
                "Ligher price too low: {}",
                order.price
            )));
        }

        // IsAsk
        if order.is_ask != 0 && order.is_ask != 1 {
            return Err(anyhow::Error::msg("Lighter IsAsk invalid"));
        }

        Ok(())
    }
}

impl TxInfo for L2CreateOrderTxInfo {
    fn get_tx_type(&self) -> u8 {
        TX_TYPE_L2_CREATE_ORDER
    }

    fn get_tx_info(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    fn get_tx_hash(&self) -> Option<String> {
        self.signed_hash.clone()
    }

    fn validate(&self) -> Result<()> {
        // Validate account index
        if self.account_index < MIN_ACCOUNT_INDEX {
            return Err(anyhow::anyhow!(
                "Lighter account index too low: {}",
                self.account_index
            ));
        }
        if self.account_index > MAX_ACCOUNT_INDEX {
            return Err(anyhow::anyhow!(
                "Lighter account index too high: {}",
                self.account_index
            ));
        }

        // Validate API key index
        if self.api_key_index > MAX_API_KEY_INDEX {
            return Err(anyhow::anyhow!(
                "Lighter API key index too high: {}",
                self.api_key_index
            ));
        }

        // Validate order info
        self.validate_order_info()?;

        // Validate nonce
        if self.nonce < MIN_NONCE {
            return Err(anyhow::anyhow!("Lighter nonce too low: {}", self.nonce));
        }

        Ok(())
    }

    fn hash(&self, _lighter_chain_id: u32) -> Result<Vec<u8>> {
        // TODO: Implement Poseidon2 hashing
        // This should hash all fields using the Goldilocks field
        Ok(vec![0u8; 40])
    }
}

pub struct LighterClient {
    pub api_client: Option<HttpClient>,
    pub chain_id: u32,
    pub account_index: i64,
    pub api_key_index: u8,
}

impl LighterClient {
    pub fn new(
        api_client_url: &str,
        account_index: i64,
        api_key_index: u8,
        chain_id: u32,
    ) -> Result<Self> {
        let api_client = if !api_client_url.is_empty() {
            Some(HttpClient::new(api_client_url)?)
        } else {
            None
        };

        Ok(LighterClient {
            api_client: api_client,
            chain_id,
            account_index,
            api_key_index,
        })
    }

    pub async fn fill_default_opts(&self, opts: Option<TransactOpts>) -> Result<TransactOpts> {
        let mut opts = opts.unwrap_or_default();

        if opts.expired_at == 0 {
            use chrono::Utc;
            // Default to 10 minutes from now
            opts.expired_at = (Utc::now().timestamp_millis() + 600_000) - 1000;
        }

        if opts.from_account_index.is_none() {
            opts.from_account_index = Some(self.account_index);
        }

        if opts.api_key_index.is_none() {
            opts.api_key_index = Some(self.api_key_index);
        }

        if opts.nonce.is_none() {
            #[derive(Deserialize)]
            struct NonceResponse {
                nonce: i64,
            }

            if opts.nonce.is_none() {
                if let Some(client) = &self.api_client {
                    let nonce = client
                        .get_next_nonce(
                            opts.from_account_index.unwrap(),
                            opts.api_key_index.unwrap(),
                        )
                        .await?;
                    opts.nonce = Some(nonce);
                } else {
                    return Err(anyhow::Error::msg(
                        "API client is not initialized to fetch nonce",
                    ));
                }
            }
        }
        Ok(opts)
    }

    pub async fn create_market_order(
        &self,
        market_index: u8,
        client_order_index: i64,
        base_amount: i64,
        price: i32,
        is_ask: u8,
        reduce_only: bool,
        opts: Option<TransactOpts>,
    ) -> Result<L2CreateOrderTxInfo> {
        let req = CreateOrderTxReq {
            market_index,
            client_order_index,
            base_amount,
            price: price as u32,
            is_ask,
            order_type: ORDER_TYPE_MARKET, // Market order
            time_in_force: TIME_IN_FORCE_IMMEDIATE_OR_CANCEL,
            reduce_only: if reduce_only { 1 } else { 0 },
            trigger_price: 0,
            order_expiry: 0,
        };

        self.create_order(req, opts).await
    }

    pub async fn create_order(
        &self,
        req: CreateOrderTxReq,
        opts: Option<TransactOpts>,
    ) -> Result<L2CreateOrderTxInfo> {
        let opts: TransactOpts = self.fill_default_opts(opts).await?;

        let order_info = OrderInfo {
            market_index: req.market_index,
            client_order_index: req.client_order_index,
            base_amount: req.base_amount,
            price: req.price,
            is_ask: req.is_ask,
            order_type: req.order_type,
            time_in_force: req.time_in_force,
            reduce_only: req.reduce_only,
            trigger_price: req.trigger_price,
            order_expiry: req.order_expiry,
        };

        let tx_info = L2CreateOrderTxInfo {
            account_index: opts.from_account_index.unwrap(),
            api_key_index: opts.api_key_index.unwrap(),
            market_index: order_info.market_index,
            client_order_index: order_info.client_order_index,
            base_amount: order_info.base_amount,
            price: order_info.price,
            is_ask: order_info.is_ask,
            order_type: order_info.order_type,
            time_in_force: order_info.time_in_force,
            reduce_only: order_info.reduce_only,
            trigger_price: order_info.trigger_price,
            order_expiry: order_info.order_expiry,
            expired_at: opts.expired_at,
            nonce: opts.nonce.unwrap(),
            sig: None,
            signed_hash: None,
            order_info,
        };

        tx_info.validate()?;

        // let msg_hash = tx_info.hash(self.chain_id)?; //TODO: should be in fe?

        //TODO: shall they go in frontend
        // let signature = self.key_manager.sign(&msg_hash); //TODO

        // tx_info.sig = Some(signature); //TODO

        // tx_info.signed_hash = Some(hex::encode(&msg_hash))?; //TODO

        Ok(tx_info)
    }
}
