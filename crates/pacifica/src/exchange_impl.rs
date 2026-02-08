//! Pacifica Exchange trait implementation.

use async_trait::async_trait;
use perp_core::{
    Exchange, PositionSide, UnifiedPosition,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange, WsMessage},
    parse_f64,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    PACIFICA_HTTP_URL, PACIFICA_WS_URL,
    apis::user::{AccountSettingsResponse, UserPositionsResponse},
    helpers::markets::PACIFICA_MARKETS,
    ws::PricesMessage,
};

/// Pacifica exchange client implementing the unified Exchange trait.
pub struct PacificaExchange {
    client: Client,
    http_url: String,
    ws_url: String,
}

impl Default for PacificaExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl PacificaExchange {
    /// Create a new Pacifica exchange client with default URLs.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            http_url: PACIFICA_HTTP_URL.to_string(),
            ws_url: PACIFICA_WS_URL.to_string(),
        }
    }

    /// Create a new Pacifica exchange client with custom URLs.
    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
        }
    }

    /// Convert a UserPositionsResponse to unified positions.
    fn convert_positions(response: UserPositionsResponse) -> Vec<UnifiedPosition> {
        let Some(positions) = response.data else {
            return Vec::new();
        };

        positions
            .into_iter()
            .filter_map(|pos| {
                let amount = parse_f64(&pos.amount)?;

                // Skip zero-size positions
                if amount.abs() < f64::EPSILON {
                    return None;
                }

                let side = match pos.side.to_lowercase().as_str() {
                    "long" => PositionSide::Long,
                    "short" => PositionSide::Short,
                    _ => return None,
                };

                Some(UnifiedPosition {
                    symbol: pos.symbol,
                    size: amount.abs(),
                    side,
                    entry_price: parse_f64(&pos.entry_price).unwrap_or(0.0),
                    mark_price: 0.0,     // Not available in this response
                    unrealized_pnl: 0.0, // Not provided in position response
                    cumulative_funding: parse_f64(&pos.funding).unwrap_or(0.0),
                    leverage: 0, // Per-position leverage, need account settings
                    margin_used: pos
                        .margin
                        .as_ref()
                        .and_then(|m| parse_f64(m))
                        .unwrap_or(0.0),
                    liquidation_price: parse_f64(&pos.liquidation_price),
                })
            })
            .collect()
    }
}

#[async_trait]
impl Exchange for PacificaExchange {
    fn name(&self) -> &'static str {
        "Pacifica"
    }

    fn perpetual_exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Pacifica
    }

    async fn get_positions(
        &self,
        user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        let response = self
            .client
            .get(format!(
                "{}/positions?account={}",
                self.http_url, user_address
            ))
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::Api {
                message: format!("HTTP {}", response.status()),
                code: Some(response.status().as_u16().to_string()),
            });
        }

        let data: UserPositionsResponse =
            response.json().await.map_err(|e| ExchangeError::Parse {
                field: "response".to_string(),
                message: e.to_string(),
            })?;

        if !data.success {
            return Err(ExchangeError::Api {
                message: data.error.unwrap_or_else(|| "Unknown error".to_string()),
                code: data.code,
            });
        }

        Ok(Self::convert_positions(data))
    }

    async fn get_account_settings(
        &self,
        user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        let response = self
            .client
            .get(format!(
                "{}/account/settings?account={}",
                self.http_url, user_address
            ))
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        let data: AccountSettingsResponse =
            response.json().await.map_err(|e| ExchangeError::Parse {
                field: "response".to_string(),
                message: e.to_string(),
            })?;

        if !data.success {
            return Err(ExchangeError::Api {
                message: data.error.unwrap_or_else(|| "Unknown error".to_string()),
                code: data.code,
            });
        }

        // Use the first setting's leverage as default if available
        let leverage = data
            .data
            .as_ref()
            .and_then(|settings| settings.first())
            .map(|s| s.leverage as u32)
            .unwrap_or(1);

        let isolated = data
            .data
            .as_ref()
            .and_then(|settings| settings.first())
            .map(|s| s.isolated)
            .unwrap_or(false);

        Ok(AccountSettings {
            leverage,
            margin_mode: Some(if isolated { "isolated" } else { "cross" }.to_string()),
            collateral: 0.0, // Not available in this endpoint
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, _symbols: &[&str]) -> Vec<String> {
        // Pacifica subscribes to all prices with a single message
        vec![
            json!({
                "method": "subscribe",
                "params": {
                    "source": "prices"
                }
            })
            .to_string(),
        ]
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        // Check for ping/pong messages first
        if raw.contains("ping") {
            return vec![WsMessage::Ping];
        }
        if raw.contains(r#""channel":"pong""#) || raw.contains(r#""channel": "pong""#) {
            return vec![WsMessage::Pong];
        }

        // Try to parse as PricesMessage
        let parsed: PricesMessage = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        // Return all valid price updates (batch processing)
        parsed
            .data
            .iter()
            .filter_map(|item| {
                let mark_price = parse_f64(&item.mark)?;
                let funding_rate = parse_f64(&item.funding)?;
                Some(WsMessage::FundingUpdate {
                    symbol: item.symbol.clone(),
                    mark_price,
                    funding_rate,
                    timestamp_ms: item.timestamp,
                })
            })
            .collect()
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        PACIFICA_MARKETS
            .iter()
            .map(|market| {
                let tick_size = parse_f64(market.tick_size).unwrap_or(0.01);
                let lot_size = parse_f64(market.lot_size).unwrap_or(0.01);

                // Calculate size decimals from lot_size
                let size_decimals = if lot_size >= 1.0 {
                    0
                } else {
                    (-lot_size.log10()).ceil() as u32
                };

                MarketInfo {
                    symbol: market.symbol.to_string(),
                    max_leverage: market.max_leverage,
                    tick_size,
                    min_order_size: parse_f64(market.min_order_size).unwrap_or(10.0),
                    size_decimals,
                    is_active: true,
                    exchange_id: None,
                }
            })
            .collect()
    }

    fn on_pong_messages(&self, _symbols: &[&str]) -> Vec<String> {
        // Re-subscribe after receiving pong to get fresh snapshot
        log::info!("Pacifica: Re-subscribing after pong");
        vec![
            json!({
                "method": "subscribe",
                "params": {
                    "source": "prices"
                }
            })
            .to_string(),
        ]
    }
}
