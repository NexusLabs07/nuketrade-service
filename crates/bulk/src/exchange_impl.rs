use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use perp_core::{
    AccountSettings, Exchange, ExchangeError, MarketInfo, PositionSide, UnifiedPosition, WsMessage,
    exchange::PerpetualExchange,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    BULK_HTTP_URL, BULK_WS_URL,
    types::{BulkAccountEntry, BulkFullAccount, BulkMarket, BulkWsMessage},
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Public Bulk Trade client used by market feeds and account-read endpoints.
///
/// This adapter intentionally supports only operations that do not require a
/// signature. Signed orders and custody operations belong in a separate layer
/// backed by Bulk's official signing implementation.
#[derive(Debug, Clone)]
pub struct BulkExchange {
    client: Client,
    http_url: String,
    ws_url: String,

    /// Normalized market metadata loaded during feed startup.
    markets: Vec<MarketInfo>,
}

impl Default for BulkExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl BulkExchange {
    /// Creates a client configured for Bulk's production HTTP and WebSocket
    /// endpoints.
    pub fn new() -> Self {
        Self::with_urls(BULK_HTTP_URL.to_string(), BULK_WS_URL.to_string())
    }

    /// Creates a client with explicit URLs.
    ///
    /// Keeping URL configuration injectable makes the adapter replaceable and
    /// allows controlled environments to point it at a compatible endpoint.
    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
            markets: Vec::new(),
        }
    }

    /// Creates the production client with previously discovered market data.
    pub fn with_markets(markets: Vec<MarketInfo>) -> Self {
        Self {
            client: Client::new(),
            http_url: BULK_HTTP_URL.to_string(),
            ws_url: BULK_WS_URL.to_string(),
            markets,
        }
    }

    /// Converts Bulk's `BTC-USD` identifier into the backend's shared `BTC`
    /// symbol namespace.
    pub fn normalize_symbol(symbol: &str) -> Option<String> {
        let normalized = symbol
            .trim()
            .strip_suffix("-USD")
            .unwrap_or(symbol.trim())
            .to_uppercase();

        (!normalized.is_empty()).then_some(normalized)
    }

    /// Normalizes an exchange timestamp into epoch milliseconds.
    ///
    /// Current Bulk ticker messages use nanoseconds. The additional magnitude
    /// checks keep the parser safe if another compatible endpoint supplies
    /// microseconds, milliseconds, or seconds.
    fn normalize_timestamp_ms(timestamp: i64) -> i64 {
        let magnitude = timestamp.unsigned_abs();

        if magnitude >= 1_000_000_000_000_000_000 {
            // Nanoseconds.
            timestamp / 1_000_000
        } else if magnitude >= 1_000_000_000_000_000 {
            // Microseconds.
            timestamp / 1_000
        } else if magnitude >= 1_000_000_000_000 {
            // Milliseconds.
            timestamp
        } else if magnitude >= 1_000_000_000 {
            // Seconds.
            timestamp * 1_000
        } else {
            timestamp
        }
    }

    /// Converts Bulk-specific market rules into the shared market type.
    fn market_info(market: &BulkMarket) -> MarketInfo {
        MarketInfo {
            symbol: Self::normalize_symbol(&market.symbol).unwrap_or_default(),
            max_leverage: market.max_leverage,
            tick_size: market.tick_size,

            // `lotSize` is Bulk's minimum base-asset order increment.
            min_order_size: market.lot_size,

            size_decimals: market.size_precision,
            is_active: market.status.eq_ignore_ascii_case("TRADING"),

            // Bulk addresses markets by symbol rather than an integer market ID.
            exchange_id: None,
        }
    }

    /// Fetches the complete public market configuration from Bulk.
    pub async fn fetch_exchange_info(&self) -> Result<Vec<BulkMarket>, ExchangeError> {
        let response = self
            .client
            .get(format!("{}/exchangeInfo", self.http_url))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| ExchangeError::Network(error.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::api(
                format!("Bulk exchangeInfo returned HTTP {}", response.status()),
                Some(response.status().as_u16().to_string()),
            ));
        }

        response
            .json()
            .await
            .map_err(|error| ExchangeError::parse("exchangeInfo", error.to_string()))
    }

    /// Fetches and normalizes every Bulk market.
    pub async fn fetch_market_info(&self) -> Result<Vec<MarketInfo>, ExchangeError> {
        Ok(self
            .fetch_exchange_info()
            .await?
            .iter()
            .map(Self::market_info)
            .filter(|market| !market.symbol.is_empty())
            .collect())
    }

    /// Returns maximum leverage keyed by the normalized backend symbol.
    pub async fn fetch_max_leverage_map(&self) -> Result<HashMap<String, u32>, ExchangeError> {
        Ok(self
            .fetch_market_info()
            .await?
            .into_iter()
            .filter(|market| market.is_active)
            .map(|market| (market.symbol, market.max_leverage))
            .collect())
    }

    /// Reads a complete public account snapshot.
    ///
    /// Bulk's `fullAccount` query is unsigned and does not grant the caller
    /// permission to mutate the requested account.
    pub async fn fetch_full_account(
        &self,
        user_address: &str,
    ) -> Result<BulkFullAccount, ExchangeError> {
        if user_address.trim().is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        let response = self
            .client
            .post(format!("{}/account", self.http_url))
            .timeout(REQUEST_TIMEOUT)
            .json(&json!({
                "type": "fullAccount",
                "user": user_address,
            }))
            .send()
            .await
            .map_err(|error| ExchangeError::Network(error.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::api(
                format!("Bulk fullAccount returned HTTP {}", response.status()),
                Some(response.status().as_u16().to_string()),
            ));
        }

        let entries: Vec<BulkAccountEntry> = response
            .json()
            .await
            .map_err(|error| ExchangeError::parse("fullAccount", error.to_string()))?;

        entries
            .into_iter()
            .find_map(|entry| entry.full_account)
            .ok_or_else(|| {
                ExchangeError::parse("fullAccount", "response did not contain fullAccount")
            })
    }

    /// Converts a signed Bulk position into the backend's unified position
    /// representation.
    fn convert_position(position: &crate::types::BulkPosition) -> Option<UnifiedPosition> {
        if !position.size.is_finite() || position.size.abs() < f64::EPSILON {
            return None;
        }

        let leverage = if position.leverage.is_finite() && position.leverage > 0.0 {
            position.leverage.floor().min(f64::from(u32::MAX)) as u32
        } else {
            0
        };

        // Bulk uses portfolio margin, so a cross position does not own an
        // independent collateral bucket. Notional divided by configured
        // leverage is used as a stable display allocation and reconstructs the
        // original notional when pair-comparison code multiplies it back by
        // leverage. Maintenance margin is used only when leverage is absent.
        let margin_used = if leverage > 0 {
            position.notional.abs() / f64::from(leverage)
        } else {
            position.maintenance_margin.max(0.0)
        };

        Some(UnifiedPosition {
            symbol: Self::normalize_symbol(&position.symbol)?,
            size: position.size.abs(),

            side: if position.size > 0.0 {
                PositionSide::Long
            } else {
                PositionSide::Short
            },

            entry_price: position.price,
            mark_price: position.fair_price,
            unrealized_pnl: position.unrealized_pnl,
            cumulative_funding: position.funding,
            leverage,
            margin_used,

            liquidation_price: position
                .liquidation_price
                .filter(|price| price.is_finite() && *price > 0.0),
        })
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
        Ok(self
            .fetch_full_account(user_address)
            .await?
            .positions
            .iter()
            .filter_map(Self::convert_position)
            .collect())
    }

    async fn get_account_settings(
        &self,
        user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
        let account = self.fetch_full_account(user_address).await?;

        Ok(AccountSettings {
            // Bulk's account response contains per-symbol leverage settings
            // rather than a meaningful account-wide leverage value.
            leverage: 0,

            margin_mode: Some("portfolio".to_string()),
            collateral: account.margin.total_balance,
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Builds one batched ticker subscription for all selected Bulk symbols.
    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        if symbols.is_empty() {
            return Vec::new();
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

    /// Parses Bulk ticker events into the shared funding update.
    ///
    /// Subscription acknowledgements, malformed messages, and unrelated
    /// streams are intentionally ignored.
    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        let parsed: BulkWsMessage = match serde_json::from_str(raw) {
            Ok(message) => message,
            Err(_) => return Vec::new(),
        };

        if parsed.message_type != "ticker" {
            return Vec::new();
        }

        let Some(ticker) = parsed.data.and_then(|data| data.ticker) else {
            return Vec::new();
        };

        if !ticker.mark_price.is_finite()
            || ticker.mark_price <= 0.0
            || !ticker.funding_rate.is_finite()
        {
            return Vec::new();
        }

        let Some(symbol) = Self::normalize_symbol(&ticker.symbol) else {
            return Vec::new();
        };

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price: ticker.mark_price,

            // Bulk publishes an hourly decimal rate, matching the shared
            // database and chart convention. No percentage conversion applies.
            funding_rate: ticker.funding_rate,

            timestamp_ms: Self::normalize_timestamp_ms(ticker.timestamp),
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        self.markets.clone()
    }
}
