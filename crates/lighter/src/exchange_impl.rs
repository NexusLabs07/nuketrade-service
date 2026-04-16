//! Lighter Exchange trait implementation.

use async_trait::async_trait;
use perp_core::{
    Exchange, UnifiedPosition, WsMessage,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange},
    parse_f64,
    token_list::TOKEN_LIST,
};
use serde_json::json;

use crate::{LIGHTER_HTTP_URL, LIGHTER_WS_URL, helpers::markets::MARKETS, types::MarketStatsMsg};

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
        Self { http_url, ws_url }
    }

    /// Find a market by its market_index.
    fn find_market_by_index(
        &self,
        market_id: u32,
    ) -> Option<&'static crate::helpers::markets::Market> {
        MARKETS.iter().find(|m| m.market_index == market_id)
    }
}

#[async_trait]
impl Exchange for LighterExchange {
    fn name(&self) -> &'static str {
        "Lighter"
    }

    fn exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Lighter
    }

    async fn get_positions(
        &self,
        _user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        // Lighter uses a different account model (zkSync-based)
        // Position fetching requires signed authentication which is not implemented
        // Return empty positions for now - to be implemented with proper auth
        Ok(Vec::new())
    }

    async fn get_account_settings(
        &self,
        _user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
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
        if raw == "ping" || raw.contains(r#""type":"ping""#) {
            return vec![WsMessage::Ping];
        }

        let parsed: MarketStatsMsg = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        if !matches!(
            parsed.lighter_type.as_str(),
            "update/market_stats" | "subscribed/market_stats"
        ) {
            return vec![];
        }

        let symbol = if !parsed.market_stats.symbol.is_empty() {
            parsed.market_stats.symbol
        } else {
            match self.find_market_by_index(parsed.market_stats.market_id) {
                Some(market) => market.symbol.to_string(),
                None => return vec![],
            }
        };

        if !TOKEN_LIST.contains(&symbol.as_str()) {
            return vec![];
        }

        let mark_price = match parse_f64(&parsed.market_stats.mark_price) {
            Some(v) => v,
            None => return vec![],
        };

        let funding_rate = parsed
            .market_stats
            .current_funding_rate
            .as_deref()
            .and_then(parse_f64)
            .or_else(|| {
                parsed
                    .market_stats
                    .funding_rate
                    .as_deref()
                    .and_then(parse_f64)
            });

        let funding_rate = match funding_rate {
            Some(v) => v,
            None => return vec![],
        };

        let timestamp_ms = parsed
            .timestamp
            .or(parsed.market_stats.funding_timestamp)
            .unwrap_or_default();

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price,
            funding_rate,
            timestamp_ms,
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

#[cfg(test)]
mod tests {
    use perp_core::{Exchange, WsMessage};
    use serde_json::{Value, json};

    use super::LighterExchange;

    #[test]
    #[test]
    fn build_subscribe_message_uses_market_index_channels() {
        let exchange = LighterExchange::new();
        let messages = exchange.build_subscribe_message(&["BTC", "ETH"]);

        assert_eq!(messages.len(), 2);

        let first: Value = serde_json::from_str(&messages[0]).expect("valid json");
        let second: Value = serde_json::from_str(&messages[1]).expect("valid json");

        assert_eq!(
            first,
            json!({
                "type": "subscribe",
                "channel": "market_stats/1"
            })
        );

        assert_eq!(
            second,
            json!({
                "type": "subscribe",
                "channel": "market_stats/0"
            })
        );
    }

    #[test]
    fn parse_market_stats_uses_current_funding_rate_and_message_timestamp() {
        let exchange = LighterExchange::new();

        let raw = r#"{
            "channel":"market_stats:1",
            "market_stats":{
                "symbol":"BTC",
                "market_id":1,
                "mark_price":"74080.2",
                "current_funding_rate":"-0.0019",
                "funding_rate":"-0.0044",
                "funding_timestamp":1776207600000
            },
            "timestamp":1776209969847,
            "type":"update/market_stats"
        }"#;

        let messages = exchange.parse_ws_message(raw);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            WsMessage::FundingUpdate {
                symbol,
                mark_price,
                funding_rate,
                timestamp_ms,
            } => {
                assert_eq!(symbol, "BTC");
                assert_eq!(*mark_price, 74080.2);
                assert_eq!(*funding_rate, -0.0019);
                assert_eq!(*timestamp_ms, 1776209969847);
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }

    #[test]
    fn parse_market_stats_ignores_unsupported_symbols() {
        let exchange = LighterExchange::new();

        let raw = r#"{
            "channel":"market_stats:96",
            "market_stats":{
                "symbol":"EURUSD",
                "market_id":96,
                "mark_price":"1.08",
                "current_funding_rate":"0.0001",
                "funding_rate":"0.0001",
                "funding_timestamp":1776207600000
            },
            "timestamp":1776209969847,
            "type":"update/market_stats"
        }"#;

        let messages = exchange.parse_ws_message(raw);
        assert!(messages.is_empty());
    }
}
