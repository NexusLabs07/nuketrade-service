use std::collections::HashMap;

use async_trait::async_trait;
use perp_core::{
    Exchange, PositionSide, UnifiedPosition, WsMessage,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange},
    parse_f64,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    PHOENIX_HTTP_URL, PHOENIX_WS_URL,
    apis::user::{PhoenixPosition, TraderStateResponse},
    helpers::markets::normalize_phoenix_symbol,
};

#[derive(Debug, Clone)]
pub struct PhoenixExchange {
    client: Client,
    http_url: String,
    ws_url: String,
    markets: Vec<MarketInfo>,
}

impl Default for PhoenixExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl PhoenixExchange {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            ws_url: PHOENIX_WS_URL.to_string(),
            markets: Vec::new(),
        }
    }

    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
            markets: Vec::new(),
        }
    }

    pub fn with_markets(markets: Vec<MarketInfo>) -> Self {
        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            ws_url: PHOENIX_WS_URL.to_string(),
            markets,
        }
    }

    pub async fn fetch_markets(&self) -> anyhow::Result<Vec<PhoenixMarket>> {
        let response = self
            .client
            .get(format!("{}/exchange/markets", self.http_url))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Phoenix markets returned HTTP {}", response.status());
        }

        Ok(response.json::<Vec<PhoenixMarket>>().await?)
    }

    pub async fn fetch_market_info(&self) -> anyhow::Result<Vec<MarketInfo>> {
        Ok(self
            .fetch_markets()
            .await?
            .into_iter()
            .map(|market| {
                let max_leverage = market
                    .leverage_tiers
                    .iter()
                    .map(|tier| tier.max_leverage.floor() as u32)
                    .max()
                    .unwrap_or(1);

                MarketInfo {
                    symbol: normalize_phoenix_symbol(&market.symbol),
                    max_leverage,
                    tick_size: market.tick_size as f64,
                    min_order_size: 0.0,
                    size_decimals: market.base_lots_decimals.max(0) as u32,
                    is_active: market.market_status == "active",
                    exchange_id: Some(market.asset_id.max(0) as u32),
                }
            })
            .collect())
    }

    pub async fn fetch_max_leverage_map(&self) -> anyhow::Result<HashMap<String, u32>> {
        Ok(self
            .fetch_market_info()
            .await?
            .into_iter()
            .map(|market| (market.symbol, market.max_leverage))
            .collect())
    }

    fn convert_positions(state: TraderStateResponse) -> Vec<UnifiedPosition> {
        state
            .traders
            .into_iter()
            .flat_map(|trader| trader.positions)
            .filter_map(Self::convert_position)
            .collect()
    }

    fn convert_position(pos: PhoenixPosition) -> Option<UnifiedPosition> {
        let size = parse_f64(&pos.position_size)?;

        if size.abs() < f64::EPSILON {
            return None;
        }

        let side = if size > 0.0 {
            PositionSide::Long
        } else {
            PositionSide::Short
        };

        Some(UnifiedPosition {
            symbol: normalize_phoenix_symbol(&pos.symbol),
            size: size.abs(),
            side,
            entry_price: parse_f64(&pos.entry_price).unwrap_or(0.0),
            mark_price: 0.0,
            unrealized_pnl: parse_f64(&pos.unrealized_pnl).unwrap_or(0.0),
            cumulative_funding: parse_f64(&pos.accumulated_funding)
                .or_else(|| parse_f64(&pos.unsettled_funding))
                .unwrap_or(0.0),
            leverage: 0,
            margin_used: parse_f64(&pos.position_initial_margin)
                .or_else(|| parse_f64(&pos.initial_margin))
                .unwrap_or(0.0),
            liquidation_price: parse_f64(&pos.liquidation_price),
        })
    }
}

#[async_trait]
impl Exchange for PhoenixExchange {
    fn name(&self) -> &'static str {
        "Phoenix"
    }

    fn exchange(&self) -> PerpetualExchange {
        PerpetualExchange::Phoenix
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
            .get(format!("{}/trader/{}/state", self.http_url, user_address))
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::Api {
                message: format!("HTTP {}", response.status()),
                code: Some(response.status().as_u16().to_string()),
            });
        }

        let state: TraderStateResponse =
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

        let response = self
            .client
            .get(format!("{}/trader/{}/state", self.http_url, user_address))
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExchangeError::Api {
                message: format!("HTTP {}", response.status()),
                code: Some(response.status().as_u16().to_string()),
            });
        }

        let state: TraderStateResponse =
            response.json().await.map_err(|e| ExchangeError::Parse {
                field: "response".to_string(),
                message: e.to_string(),
            })?;

        let collateral = state
            .traders
            .first()
            .and_then(|trader| parse_f64(&trader.effective_collateral))
            .unwrap_or(0.0);

        Ok(AccountSettings {
            leverage: 0,
            margin_mode: Some("cross".to_string()),
            collateral,
        })
    }

    fn ws_url(&self) -> &str {
        &self.ws_url
    }

    fn build_subscribe_message(&self, symbols: &[&str]) -> Vec<String> {
        symbols
            .iter()
            .map(|symbol| {
                json!({
                    "type": "subscribe",
                    "subscription": {
                        "channel": "market",
                        "symbol": normalize_phoenix_symbol(symbol)
                    }
                })
                .to_string()
            })
            .collect()
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        let parsed: PhoenixMarketStatsMessage = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        if parsed.channel != "market" {
            return vec![];
        }

        let Some(mark_px) = parsed.mark_px else {
            return vec![];
        };

        let Some(funding) = parsed.funding else {
            return vec![];
        };

        vec![WsMessage::FundingUpdate {
            symbol: normalize_phoenix_symbol(&parsed.symbol),
            mark_price: mark_px,
            funding_rate: funding,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }]
    }

    fn get_markets(&self) -> Vec<MarketInfo> {
        self.markets.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixMarket {
    pub asset_id: i32,
    pub base_lots_decimals: i32,
    pub funding_interval_seconds: i32,
    pub funding_period_seconds: i32,
    pub isolated_only: bool,
    #[serde(default)]
    pub leverage_tiers: Vec<PhoenixLeverageTier>,
    pub market_pubkey: String,
    pub market_status: String,
    pub symbol: String,
    pub tick_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixLeverageTier {
    pub limit_order_risk_factor: f64,
    pub max_leverage: f64,
    pub max_size_base_lots: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoenixMarketStatsMessage {
    pub channel: String,
    #[serde(default)]
    pub symbol: String,
    pub open_interest: Option<f64>,
    pub mark_px: Option<f64>,
    pub mid_px: Option<f64>,
    pub oracle_px: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub day_ntl_vlm: Option<f64>,
    pub funding: Option<f64>,
}
