//! Exchange trait abstraction for unified exchange operations.
//!
//! This module provides a common interface for all supported exchanges,
//! enabling polymorphic handling of exchange-specific operations.

use crate::funding::Dex;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unified position representation across all exchanges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPosition {
    pub symbol: String,
    pub size: f64,
    pub side: PositionSide,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub cumulative_funding: f64,
    pub leverage: u32,
    pub margin_used: f64,
    pub liquidation_price: Option<f64>,
}

/// Position side enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "long"),
            PositionSide::Short => write!(f, "short"),
        }
    }
}

/// Market metadata that varies per exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub symbol: String,
    pub max_leverage: u32,
    pub tick_size: f64,
    pub min_order_size: f64,
    pub size_decimals: u32,
    pub is_active: bool,
    /// Exchange-specific identifier (e.g., Lighter's market_index)
    pub exchange_id: Option<u32>,
}

/// Account settings returned from exchanges.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountSettings {
    pub leverage: u32,
    pub margin_mode: Option<String>,
    pub collateral: f64,
}

/// WebSocket message types for funding feeds.
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Funding rate update with market data
    FundingUpdate {
        symbol: String,
        mark_price: f64,
        funding_rate: f64,
        timestamp_ms: i64,
    },
    /// Ping message for keepalive
    Ping,
    /// Pong response
    Pong,
    /// Connection closed
    Close,
    /// Unrecognized message
    Unknown(String),
}

/// Exchange-specific errors with detailed context.
#[derive(Debug, Error)]
pub enum ExchangeError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error in field '{field}': {message}")]
    Parse { field: String, message: String },

    #[error("API error: {message} (code: {code:?})")]
    Api { message: String, code: Option<String> },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Address required but not provided")]
    AddressRequired,

    #[error("Market not found: {0}")]
    MarketNotFound(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl ExchangeError {
    /// Create a parse error with field context.
    pub fn parse(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an API error.
    pub fn api(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Api {
            message: message.into(),
            code,
        }
    }
}

/// Core exchange trait defining unified interface for all exchanges.
///
/// Implementations should handle exchange-specific API formats and convert
/// them to the unified types defined in this module.
#[async_trait]
pub trait Exchange: Send + Sync {
    /// Returns the human-readable exchange name.
    fn name(&self) -> &'static str;

    /// Returns the DEX enum variant for this exchange.
    fn dex(&self) -> Dex;

    /// Fetches open positions for a user.
    ///
    /// # Arguments
    /// * `user_address` - The user's address (EVM or Solana depending on exchange)
    ///
    /// # Returns
    /// A vector of unified positions or an exchange error.
    async fn get_positions(&self, user_address: &str) -> Result<Vec<UnifiedPosition>, ExchangeError>;

    /// Fetches account settings for a user.
    ///
    /// # Arguments
    /// * `user_address` - The user's address
    ///
    /// # Returns
    /// Account settings or an exchange error.
    async fn get_account_settings(&self, user_address: &str) -> Result<AccountSettings, ExchangeError>;

    /// Returns the WebSocket URL for this exchange.
    fn ws_url(&self) -> &str;

    /// Builds subscription messages for the given symbols.
    ///
    /// # Arguments
    /// * `symbols` - List of market symbols to subscribe to
    ///
    /// # Returns
    /// A vector of JSON strings to send to the WebSocket.
    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String>;

    /// Parses a raw WebSocket message into typed messages.
    ///
    /// # Arguments
    /// * `raw` - The raw message string from WebSocket
    ///
    /// # Returns
    /// A vector of typed WsMessage variants. Returns empty vec for unrecognized messages.
    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage>;

    /// Returns the list of available markets.
    fn get_markets(&self) -> Vec<MarketInfo>;

    /// Looks up a specific market by symbol.
    fn get_market(&self, symbol: &str) -> Option<MarketInfo> {
        self.get_markets().into_iter().find(|m| m.symbol == symbol)
    }

    /// Returns messages to send when a pong is received.
    ///
    /// Some exchanges (e.g., Pacifica) need to re-subscribe after receiving a pong
    /// to ensure fresh data. Default implementation returns an empty vec.
    fn on_pong_messages(&self, _symbols: &[&str]) -> Vec<String> {
        vec![]
    }
}
