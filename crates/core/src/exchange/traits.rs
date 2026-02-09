//! Core exchange trait defining the unified interface for all exchanges.

use async_trait::async_trait;

use super::error::ExchangeError;
use super::types::PerpetualExchange;
use crate::types::{AccountSettings, MarketInfo, UnifiedPosition};
use crate::ws::WsMessage;

/// Core exchange trait defining unified interface for all exchanges.
///
/// Implementations should handle exchange-specific API formats and convert
/// them to the unified types defined in this crate.
#[async_trait]
pub trait Exchange: Send + Sync {
    /// Returns the human-readable exchange name.
    fn name(&self) -> &'static str;

    /// Returns the exchange enum variant for this exchange.
    fn exchange(&self) -> PerpetualExchange;

    /// Fetches open positions for a user.
    ///
    /// # Arguments
    /// * `user_address` - The user's address (EVM or Solana depending on exchange)
    ///
    /// # Returns
    /// A vector of unified positions or an exchange error.
    async fn get_positions(
        &self,
        user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError>;

    /// Fetches account settings for a user.
    ///
    /// # Arguments
    /// * `user_address` - The user's address
    ///
    /// # Returns
    /// Account settings or an exchange error.
    async fn get_account_settings(
        &self,
        user_address: &str,
    ) -> Result<AccountSettings, ExchangeError>;

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
