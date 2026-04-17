//! Lighter Exchange trait implementation.

use std::collections::HashMap;

use async_trait::async_trait;
use perp_core::{
    Exchange, UnifiedPosition, WsMessage,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange},
    parse_f64,
    token_list::TOKEN_LIST,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    LIGHTER_HTTP_URL, LIGHTER_WS_URL,
    helpers::markets::{LighterPerpMarket, perp_market_from_order_book_detail},
    types::{MarketStatsMsg, OrderBookDetailsResponse},
};

fn parse_lighter_percentage_rate(raw: &str) -> Option<f64> {
    // Lighter exposes funding fields in percentage units
    parse_f64(raw).map(|pct| pct / 100.0)
}

/// Lighter exchange client implementing the unified Exchange trait.
pub struct LighterExchange {
    client: Client,
    #[allow(dead_code)]
    http_url: String,
    ws_url: String,
    markets: Vec<LighterPerpMarket>,
    markets_by_symbol: HashMap<String, LighterPerpMarket>,
    market_by_index: HashMap<u32, LighterPerpMarket>,
}

impl Default for LighterExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl LighterExchange {
    /// Create a new Lighter exchange client with default URLs and no cached markets.
    pub fn new() -> Self {
        Self::with_urls_and_markets(
            LIGHTER_HTTP_URL.to_string(),
            LIGHTER_WS_URL.to_string(),
            Vec::new(),
        )
    }

    /// Create a new Lighter exchange client with default URLs and known markets.
    pub fn with_markets(markets: Vec<LighterPerpMarket>) -> Self {
        Self::with_urls_and_markets(
            LIGHTER_HTTP_URL.to_string(),
            LIGHTER_WS_URL.to_string(),
            markets,
        )
    }

    /// Create a new Lighter exchange client with custom URLs and no cached markets.
    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self::with_urls_and_markets(http_url, ws_url, Vec::new())
    }

    fn with_urls_and_markets(
        http_url: String,
        ws_url: String,
        markets: Vec<LighterPerpMarket>,
    ) -> Self {
        let markets_by_symbol = markets
            .iter()
            .cloned()
            .map(|market| (market.symbol.clone(), market))
            .collect();

        let market_by_index = markets
            .iter()
            .cloned()
            .map(|market| (market.market_index, market))
            .collect();

        Self {
            client: Client::new(),
            http_url,
            ws_url,
            markets,
            markets_by_symbol,
            market_by_index,
        }
    }

    pub async fn fetch_active_perp_markets(&self) -> Result<Vec<LighterPerpMarket>, ExchangeError> {
        let response = self
            .client
            .get(format!("{}/api/v1/orderBookDetails", self.http_url))
            .query(&[("filter", "perp")])
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::api(
                format!("HTTP {}", response.status()),
                Some(response.status().as_u16().to_string()),
            ));
        }

        let payload: OrderBookDetailsResponse = response
            .json()
            .await
            .map_err(|e| ExchangeError::parse("orderBookDetails", e.to_string()))?;

        let mut markets: Vec<LighterPerpMarket> = payload
            .order_book_details
            .iter()
            .filter_map(perp_market_from_order_book_detail)
            .collect();

        markets.sort_unstable_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(markets)
    }

    fn find_market_by_index(&self, market_id: u32) -> Option<&LighterPerpMarket> {
        self.market_by_index.get(&market_id)
    }

    fn find_market_by_symbol(&self, symbol: &str) -> Option<&LighterPerpMarket> {
        self.markets_by_symbol.get(symbol)
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
                let market = self.find_market_by_symbol(symbol)?;
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
                Some(market) => market.symbol.clone(),
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
            .and_then(parse_lighter_percentage_rate)
            .or_else(|| {
                parsed
                    .market_stats
                    .funding_rate
                    .as_deref()
                    .and_then(parse_lighter_percentage_rate)
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
        self.markets
            .iter()
            .map(|market| MarketInfo {
                symbol: market.symbol.clone(),
                max_leverage: market.max_leverage,
                tick_size: market.tick_size,
                min_order_size: market.min_order_size,
                size_decimals: market.size_decimals,
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
    use crate::helpers::markets::LighterPerpMarket;

    fn test_exchange() -> LighterExchange {
        LighterExchange::with_markets(vec![
            LighterPerpMarket {
                symbol: "ETH".to_string(),
                market_index: 0,
                tick_size: 0.01,
                min_order_size: 0.0001,
                size_decimals: 4,
                max_leverage: 50,
            },
            LighterPerpMarket {
                symbol: "BTC".to_string(),
                market_index: 1,
                tick_size: 0.1,
                min_order_size: 0.00001,
                size_decimals: 5,
                max_leverage: 50,
            },
        ])
    }

    #[test]
    fn build_subscribe_message_uses_market_index_channels() {
        let exchange = test_exchange();
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
        let exchange = test_exchange();

        let raw = r#"{
            "channel":"market_stats:1",
            "market_stats":{
                "symbol":"BTC",
                "market_id":1,
                "mark_price":"74080.2",
                "current_funding_rate":"0.0012",
                "funding_rate":"0.0011",
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
                assert!((*funding_rate - 0.000012).abs() < 1e-12);
                assert_eq!(*timestamp_ms, 1776209969847);
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }

    #[test]
    fn parse_market_stats_falls_back_to_market_id_when_symbol_missing() {
        let exchange = test_exchange();

        let raw = r#"{
            "channel":"market_stats:1",
            "market_stats":{
                "symbol":"",
                "market_id":1,
                "mark_price":"74080.2",
                "current_funding_rate":"0.0012",
                "funding_rate":"0.0011",
                "funding_timestamp":1776207600000
            },
            "timestamp":1776209969847,
            "type":"update/market_stats"
        }"#;

        let messages = exchange.parse_ws_message(raw);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            WsMessage::FundingUpdate { symbol, .. } => {
                assert_eq!(symbol, "BTC");
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }

    #[test]
    fn parse_market_stats_ignores_unsupported_symbols() {
        let exchange = test_exchange();

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
