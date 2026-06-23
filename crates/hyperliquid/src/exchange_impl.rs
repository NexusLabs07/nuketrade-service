//! Hyperliquid Exchange trait implementation.

use async_trait::async_trait;
use perp_core::{
    Exchange, PositionSide, UnifiedPosition, WsMessage,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange},
    parse_f64,
};
use reqwest::Client;
use serde_json::json;

use crate::{
    HYPERLIQUID_HTTP_URL, HYPERLIQUID_WS_URL,
    apis::user::{ClearinghouseState, OpenPositionRequest},
    helpers::markets::{HL_MARKETS, normalize_hl_symbol, subscription_coin},
    types::ActiveAssetCtxMsg,
};

/// Hyperliquid exchange client implementing the unified Exchange trait.
pub struct HyperliquidExchange {
    client: Client,
    http_url: String,
    ws_url: String,
}

impl Default for HyperliquidExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperliquidExchange {
    /// Create a new Hyperliquid exchange client with default URLs.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            http_url: HYPERLIQUID_HTTP_URL.to_string(),
            ws_url: HYPERLIQUID_WS_URL.to_string(),
        }
    }

    /// Create a new Hyperliquid exchange client with custom URLs.
    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
        }
    }

    /// Convert a ClearinghouseState response to unified positions.
    fn convert_positions(state: ClearinghouseState) -> Vec<UnifiedPosition> {
        state
            .asset_positions
            .into_iter()
            .filter_map(|ap| {
                let pos = ap.position;
                let size = parse_f64(&pos.szi)?;

                // Skip zero-size positions
                if size.abs() < f64::EPSILON {
                    return None;
                }

                let side = if size > 0.0 {
                    PositionSide::Long
                } else {
                    PositionSide::Short
                };

                let symbol = normalize_hl_symbol(&pos.coin);

                Some(UnifiedPosition {
                    symbol,
                    size: size.abs(),
                    side,
                    entry_price: parse_f64(&pos.entry_px).unwrap_or(0.0),
                    mark_price: 0.0,
                    unrealized_pnl: parse_f64(&pos.unrealized_pnl).unwrap_or(0.0),
                    // HL cumFunding is inverted vs trader cashflow (+ = received, − = paid).
                    cumulative_funding: -parse_f64(&pos.cum_funding.all_time).unwrap_or(0.0),
                    leverage: pos.leverage.value,
                    margin_used: pos.collateral_margin_usd(),
                    liquidation_price: pos.liquidation_px.as_ref().and_then(|px| parse_f64(px)),
                })
            })
            .collect()
    }
}

#[async_trait]
impl Exchange for HyperliquidExchange {
    fn name(&self) -> &'static str {
        "Hyperliquid"
    }

    fn exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Hyperliquid
    }

    async fn get_positions(
        &self,
        user_address: &str,
    ) -> Result<Vec<UnifiedPosition>, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        let request = OpenPositionRequest {
            position_type: "clearinghouseState".to_string(),
            user: user_address.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/info", self.http_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::Api {
                message: format!("HTTP {}", response.status()),
                code: Some(response.status().as_u16().to_string()),
            });
        }

        let state: ClearinghouseState =
            response.json().await.map_err(|e| ExchangeError::Parse {
                field: "response".to_string(),
                message: e.to_string(),
            })?;

        Ok(Self::convert_positions(state))
    }

    async fn get_account_settings(
        &self,
        user_address: &str,
    ) -> Result<AccountSettings, ExchangeError> {
        if user_address.is_empty() {
            return Err(ExchangeError::AddressRequired);
        }

        let request = OpenPositionRequest {
            position_type: "clearinghouseState".to_string(),
            user: user_address.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/info", self.http_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        let state: ClearinghouseState =
            response.json().await.map_err(|e| ExchangeError::Parse {
                field: "response".to_string(),
                message: e.to_string(),
            })?;

        Ok(AccountSettings {
            leverage: 0, // Hyperliquid uses per-position leverage
            margin_mode: Some("cross".to_string()),
            collateral: parse_f64(&state.cross_margin_summary.account_value).unwrap_or(0.0),
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        symbols
            .iter()
            .map(|symbol| {
                let coin = subscription_coin(symbol);

                json!({
                    "method": "subscribe",
                    "subscription": {
                        "type": "activeAssetCtx",
                        "coin": coin
                    }
                })
                .to_string()
            })
            .collect()
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        // Try to parse as ActiveAssetCtxMsg
        let parsed: ActiveAssetCtxMsg = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let symbol = normalize_hl_symbol(&parsed.data.coin);

        let funding_rate = match parse_f64(&parsed.data.ctx.funding) {
            Some(v) => v,
            None => return vec![],
        };

        let mark_price = match parse_f64(&parsed.data.ctx.mark_px) {
            Some(v) => v,
            None => return vec![],
        };

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price,
            funding_rate,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        HL_MARKETS
            .iter()
            .map(|asset| MarketInfo {
                symbol: asset.display_symbol().to_string(),
                max_leverage: asset.max_leverage,
                tick_size: 10_f64.powi(-(asset.sz_decimals as i32)),
                min_order_size: 10_f64.powi(-(asset.sz_decimals as i32)),
                size_decimals: asset.sz_decimals,
                is_active: !asset.is_delisted,
                exchange_id: Some(asset.margin_table_id),
            })
            .collect()
    }
}
