use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use perp_core::{
    AccountSettings, Exchange, ExchangeError, MarketInfo, UnifiedPosition, WsMessage,
    exchange::PerpetualExchange, parse_f64,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    BACKPACK_HTTP_URL, BACKPACK_WS_URL,
    helpers::markets::{
        canonical_symbol_from_backpack_symbol, is_active_perp_market, max_leverage_from_market,
    },
    types::{BackpackMarket, MarkPriceData, StreamEnvelope},
};

/// Backpack exchange client for public funding feed
///
/// currently contains
/// - active PERP symbols
/// - subscribing to `markPrice.<symbol>` websocket streams
/// - normalizing Backpack symbols into our symbol format
pub struct BackpackExchange {
    client: Client,
    http_url: String,
    ws_url: String,
}

impl Default for BackpackExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl BackpackExchange {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            http_url: BACKPACK_HTTP_URL.to_string(),
            ws_url: BACKPACK_WS_URL.to_string(),
        }
    }

    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
        }
    }

    pub async fn fetch_active_markets(&self) -> Result<Vec<BackpackMarket>, ExchangeError> {
        let response = self
            .client
            .get(format!("{}/markets", self.http_url))
            .query(&[("marketType", "PERP")])
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::api(
                format!("HTTP {}", response.status()),
                Some(response.status().as_u16().to_string()),
            ));
        }

        let markets: Vec<BackpackMarket> = response
            .json()
            .await
            .map_err(|e| ExchangeError::parse("markets", e.to_string()))?;

        Ok(markets.into_iter().filter(is_active_perp_market).collect())
    }

    /// Finding visible active PERP symbols from Backpack REST
    /// Using Backpack's REST discovery because symbols can vary (`*_USDC`, `*_USDC_PERP`, etc.), and this keeps the
    /// websocket bootstrap accurate
    pub async fn fetch_active_funding_symbols(&self) -> Result<Vec<String>, ExchangeError> {
        let symbols = self
            .fetch_active_markets()
            .await?
            .into_iter()
            .map(|market| market.symbol)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        Ok(symbols)
    }

    pub async fn fetch_max_leverage_map(&self) -> Result<HashMap<String, u32>, ExchangeError> {
        let mut leverage_by_symbol = HashMap::new();

        for market in self.fetch_active_markets().await? {
            let Some(symbol) = canonical_symbol_from_backpack_symbol(&market.symbol) else {
                continue;
            };

            let Some(max_leverage) = max_leverage_from_market(&market) else {
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
impl Exchange for BackpackExchange {
    fn name(&self) -> &'static str {
        "Backpack"
    }

    fn exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Backpack
    }

    async fn get_positions(
        &self,
        _user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        // Backpack account intergration needs ED25519 auth so i am thinking of implementing it seperately outside of websocket
        Ok(Vec::new())
    }

    async fn get_account_settings(
        &self,
        _user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
        Ok(AccountSettings {
            leverage: 0,
            margin_mode: Some("cross".to_string()),
            collateral: 0.0,
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        let params: Vec<String> = symbols
            .iter()
            .map(|symbol| format!("markPrice.{symbol}"))
            .collect();

        if params.is_empty() {
            return vec![];
        }

        vec![
            json!({
                "method": "SUBSCRIBE",
                "params": params,
            })
            .to_string(),
        ]
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        let parsed: StreamEnvelope<MarkPriceData> = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(_) => return vec![],
        };

        if !parsed.stream.starts_with("markPrice.") {
            return vec![];
        }

        if parsed.data.event_type != "markPrice" {
            return vec![];
        }

        let symbol = match canonical_symbol_from_backpack_symbol(&parsed.data.symbol) {
            Some(symbol) => symbol,
            None => return vec![],
        };

        let mark_price = match parse_f64(&parsed.data.mark_price) {
            Some(value) => value,
            None => return vec![],
        };

        let funding_rate = match parsed.data.funding_rate.as_deref().and_then(parse_f64) {
            Some(value) => value,
            None => return vec![],
        };

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price,
            funding_rate,
            timestamp_ms: parsed.data.event_time_us / 1000,
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        // for now only keeping REST symbol discovery and websocket parsing
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use perp_core::{Exchange, WsMessage};
    use serde_json::{Value, json};

    use super::BackpackExchange;

    #[test]
    fn build_subscribe_message_batches_multiple_streams() {
        let exchange = BackpackExchange::new();
        let messages = exchange.build_subscribe_message(&["BTC_USDC", "SOL_USDC"]);

        assert_eq!(messages.len(), 1);

        let parsed: Value = serde_json::from_str(&messages[0]).expect("valid json");
        assert_eq!(
            parsed,
            json!({
                "method": "SUBSCRIBE",
                "params": [
                    "markPrice.BTC_USDC",
                    "markPrice.SOL_USDC"
                ]
            })
        );
    }

    #[test]
    fn parse_mark_price_message_into_funding_update() {
        let exchange = BackpackExchange::new();

        let raw = r#"{
            "stream": "markPrice.SOL_USDC",
            "data": {
                "e": "markPrice",
                "E": 1694687965941000,
                "s": "SOL_USDC",
                "p": "18.70",
                "f": "0.0001",
                "i": "19.70",
                "n": 1694687965941,
                "T": 1694687965940999
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
                assert_eq!(*mark_price, 18.70);
                assert_eq!(*funding_rate, 0.0001);
                assert_eq!(*timestamp_ms, 1694687965941);
            }
            other => panic!("unexpected ws message: {other:?}"),
        }
    }
}
