//! Lighter Exchange trait implementation.

use async_trait::async_trait;
use perp_core::{
    exchange::{AccountSettings, ExchangeError, MarketInfo, WsMessage},
    funding::Dex,
    parse_f64, Exchange, UnifiedPosition,
};
use serde_json::json;

use crate::{
    helpers::markets::MARKETS,
    types::MarketStatsMsg,
    LIGHTER_HTTP_URL, LIGHTER_WS_URL,
};

/// Lighter exchange client implementing the unified Exchange trait.
pub struct LighterExchange {
    #[allow(dead_code)]
    http_url: String,
    ws_url: String,
}

impl Default for LighterExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl LighterExchange {
    /// Create a new Lighter exchange client with default URLs.
    pub fn new() -> Self {
        Self {
            http_url: LIGHTER_HTTP_URL.to_string(),
            ws_url: LIGHTER_WS_URL.to_string(),
        }
    }

    /// Create a new Lighter exchange client with custom URLs.
    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            http_url,
            ws_url,
        }
    }

    /// Find a market by its market_index.
    fn find_market_by_index(&self, market_id: u32) -> Option<&'static crate::helpers::markets::Market> {
        MARKETS.iter().find(|m| m.market_index == market_id)
    }
}

#[async_trait]
impl Exchange for LighterExchange {
    fn name(&self) -> &'static str {
        "Lighter"
    }

    fn dex(&self) -> Dex {
        Dex::Lighter
    }

    async fn get_positions(&self, _user_address: &str) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        // Lighter uses a different account model (zkSync-based)
        // Position fetching requires signed authentication which is not implemented
        // Return empty positions for now - to be implemented with proper auth
        Ok(Vec::new())
    }

    async fn get_account_settings(&self, _user_address: &str) -> Result<AccountSettings, ExchangeError> {
        // Lighter uses a different account model
        // Return default settings for now
        Ok(AccountSettings {
            leverage: 10,
            margin_mode: Some("cross".to_string()),
            collateral: 0.0,
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        symbols
            .iter()
            .filter_map(|symbol| {
                // Find the market index for this symbol
                let market = MARKETS.iter().find(|m| m.symbol == *symbol)?;
                Some(
                    json!({
                        "type": "subscribe",
                        "channel": format!("market_stats/{}", market.market_index)
                    })
                    .to_string(),
                )
            })
            .collect()
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        // Check for ping messages
        if raw.contains("ping") {
            return vec![WsMessage::Ping];
        }

        // Try to parse as MarketStatsMsg
        let parsed: MarketStatsMsg = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        // Find the market by index
        let market = match self.find_market_by_index(parsed.market_stats.market_id) {
            Some(m) => m,
            None => return vec![],
        };

        let mark_price = match parse_f64(&parsed.market_stats.mark_price) {
            Some(v) => v,
            None => return vec![],
        };

        // Lighter returns 8-hour funding rate, convert to hourly
        let funding_8h = match parse_f64(&parsed.market_stats.funding_rate) {
            Some(v) => v,
            None => return vec![],
        };
        let funding_rate = funding_8h / 8.0;

        vec![WsMessage::FundingUpdate {
            symbol: market.symbol.to_string(),
            mark_price,
            funding_rate,
            timestamp_ms: (parsed.market_stats.funding_timestamp * 1000) as i64,
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        MARKETS
            .iter()
            .map(|market| MarketInfo {
                symbol: market.symbol.to_string(),
                max_leverage: 20, // Default max leverage for Lighter
                tick_size: 0.01,  // Default tick size
                min_order_size: 1.0,
                size_decimals: 2,
                is_active: true,
                exchange_id: Some(market.market_index),
            })
            .collect()
    }
}
