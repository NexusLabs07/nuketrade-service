use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use perp_core::{
    AccountSettings, Exchange, ExchangeError, MarketInfo, UnifiedPosition, WsMessage,
    exchange::PerpetualExchange,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    BULK_HTTP_URL, BULK_WS_URL,
    helpers::markets::{canonical_symbol_from_bulk_symbol, normalize_timestamp_ms},
    types::{BulkMarket, BulkTicker, BulkTickerData, BulkWsMessage},
};

pub struct BulkExchange {
    client: Client,
    http_url: String,
    ws_url: String,
}

impl Default for BulkExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl BulkExchange {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            http_url: BULK_HTTP_URL.to_string(),
            ws_url: BULK_WS_URL.to_string(),
        }
    }

    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
        }
    }

    pub async fn fetch_exchange_info(&self) -> Result<Vec<BulkMarket>, ExchangeError> {
        let response = self
            .client
            .get(format!("{}/exchangeInfo", self.http_url))
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::api(
                format!("HTTP {}", response.status()),
                Some(response.status().as_u16().to_string()),
            ));
        }

        response
            .json()
            .await
            .map_err(|e| ExchangeError::parse("exchangeInfo", e.to_string()))
    }

    pub async fn fetch_active_markets(&self) -> Result<Vec<BulkMarket>, ExchangeError> {
        Ok(self
            .fetch_exchange_info()
            .await?
            .into_iter()
            .filter(|market| market.status.eq_ignore_ascii_case("TRADING"))
            .collect())
    }

    pub async fn fetch_active_funding_symbols(&self) -> Result<Vec<String>, ExchangeError> {
        Ok(self
            .fetch_active_markets()
            .await?
            .into_iter()
            .map(|market| market.symbol)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect())
    }

    pub async fn fetch_max_leverage_map(&self) -> Result<HashMap<String, u32>, ExchangeError> {
        let mut leverage_by_symbol = HashMap::new();

        for market in self.fetch_active_markets().await? {
            let Some(symbol) = canonical_symbol_from_bulk_symbol(&market.symbol) else {
                continue;
            };

            let Some(max_leverage) = market.max_leverage else {
                continue;
            };

            leverage_by_symbol
                .entry(symbol)
                .and_modify(|current: &mut u32| *current = (*current).max(max_leverage))
                .or_insert(max_leverage);
        }

        Ok(leverage_by_symbol)
    }
}

#[async_trait]
impl Exchange for BulkExchange {
    fn name(&self) -> &'static str {
        "Bulk"
    }

    fn exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Bulk
    }

    async fn get_positions(
        &self,
        user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        Ok(Vec::new())
    }

    async fn get_account_settings(
        &self,
        user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        Ok(AccountSettings {
            leverage: 0,
            margin_mode: Some("portfolio".to_string()),
            collateral: 0.0,
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        if symbols.is_empty() {
            return vec![];
        }

        let subscription: Vec<_> = symbols
            .iter()
            .map(|symbol| {
                json!({
                    "type": "ticker",
                    "symbol": symbol,
                })
            })
            .collect();

        vec![
            json!({
                "method": "subscribe",
                "subscription": subscription,
            })
            .to_string(),
        ]
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        let parsed: BulkWsMessage = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(_) => return vec![],
        };

        if parsed.message_type != "ticker" {
            return vec![];
        }

        let topic = parsed.topic.clone();
        let outer_symbol = parsed.symbol.clone();

        let BulkTickerData {
            symbol: data_symbol,
            ticker,
            mark_price: data_mark_price,
            funding_rate: data_funding_rate,
            timestamp: data_timestamp,
        } = parsed.data.unwrap_or_default();

        let BulkTicker {
            symbol: ticker_symbol,
            mark_price: ticker_mark_price,
            funding_rate: ticker_funding_rate,
            timestamp: ticker_timestamp,
        } = ticker.unwrap_or_default();

        let raw_symbol = ticker_symbol.or(data_symbol).or(outer_symbol).or_else(|| {
            topic.as_deref().and_then(|value| {
                value
                    .strip_prefix("ticker.")
                    .map(std::string::ToString::to_string)
            })
        });

        let symbol = match raw_symbol
            .as_deref()
            .and_then(canonical_symbol_from_bulk_symbol)
        {
            Some(symbol) => symbol,
            None => return vec![],
        };

        let mark_price = match ticker_mark_price.or(data_mark_price) {
            Some(value) => value,
            None => return vec![],
        };

        let funding_rate = match ticker_funding_rate.or(data_funding_rate) {
            Some(value) => value,
            None => return vec![],
        };

        let timestamp_ms = match ticker_timestamp.or(data_timestamp) {
            Some(value) => normalize_timestamp_ms(value),
            None => return vec![],
        };

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price,
            funding_rate,
            timestamp_ms,
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        // Dynamic REST discovery is used for the websocket bootstrap.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use perp_core::{Exchange, WsMessage};
    use serde_json::{Value, json};

    use super::BulkExchange;

    #[test]
    fn build_subscribe_message_batches_multiple_streams() {
        let exchange = BulkExchange::new();
        let messages = exchange.build_subscribe_message(&["BTC-USD", "SOL-USD"]);

        assert_eq!(messages.len(), 1);

        let parsed: Value = serde_json::from_str(&messages[0]).expect("valid json");
        assert_eq!(
            parsed,
            json!({
                "method": "subscribe",
                "subscription": [
                    { "type": "ticker", "symbol": "BTC-USD" },
                    { "type": "ticker", "symbol": "SOL-USD" }
                ]
            })
        );
    }

    #[test]
    fn parse_ticker_message_into_funding_update() {
        let exchange = BulkExchange::new();

        let raw = r#"{
            "type": "ticker",
            "data": {
                "ticker": {
                    "symbol": "SOL-USD",
                    "markPrice": 152.45,
                    "fundingRate": 0.0001,
                    "timestamp": 1763316177219383423
                }
            }
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
                assert_eq!(symbol, "SOL");
                assert_eq!(*mark_price, 152.45);
                assert_eq!(*funding_rate, 0.0001);
                assert_eq!(*timestamp_ms, 1763316177219);
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }
}
