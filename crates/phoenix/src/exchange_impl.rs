use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use perp_core::{
    Exchange, PositionSide, UnifiedPosition, WsMessage,
    exchange::{AccountSettings, ExchangeError, MarketInfo, PerpetualExchange},
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
    /// Symbols subscribed by the funding feed.
    subscribed_symbols: Arc<HashSet<String>>,
    /// Latest mid prices from the `allMids` channel, keyed by normalized symbol.
    latest_mids: Arc<Mutex<HashMap<String, f64>>>,
}

impl Default for PhoenixExchange {
    fn default() -> Self {
        Self::new()
    }
}

impl PhoenixExchange {
    pub fn new() -> Self {
        Self::with_markets(Vec::new())
    }

    pub fn with_urls(http_url: String, ws_url: String) -> Self {
        Self {
            client: Client::new(),
            http_url,
            ws_url,
            markets: Vec::new(),
            subscribed_symbols: Arc::new(HashSet::new()),
            latest_mids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_markets(markets: Vec<MarketInfo>) -> Self {
        Self::with_feed(markets, &[])
    }

    /// Funding feed: `allMids` (one sub) + `fundingRate` per symbol (see Phoenix WS docs).
    pub fn with_feed(markets: Vec<MarketInfo>, symbols: &[String]) -> Self {
        let subscribed_symbols: HashSet<String> = symbols
            .iter()
            .map(|s| normalize_phoenix_symbol(s))
            .collect();

        Self {
            client: Client::new(),
            http_url: PHOENIX_HTTP_URL.to_string(),
            ws_url: PHOENIX_WS_URL.to_string(),
            markets,
            subscribed_symbols: Arc::new(subscribed_symbols),
            latest_mids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Phoenix WS `funding` is the interval rate as a **percent** of notional
    /// (e.g. `-0.0075` means −0.0075%/hour), not a decimal fraction like Hyperliquid.
    fn normalize_funding_to_hourly_decimal(funding_percent: f64) -> f64 {
        funding_percent / 100.0
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

    fn market_info_from_raw(market: &PhoenixMarket) -> MarketInfo {
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
    }

    pub async fn fetch_market_info(&self) -> anyhow::Result<Vec<MarketInfo>> {
        Ok(self
            .fetch_markets()
            .await?
            .iter()
            .map(Self::market_info_from_raw)
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
        let mut out = Vec::new();
        for trader in state.traders {
            let collateral = trader.collateral_usd();
            let single_position = trader.positions.len() == 1;
            for pos in trader.positions {
                if let Some(unified) = Self::convert_position(&pos, collateral, single_position) {
                    out.push(unified);
                }
            }
        }
        out
    }

    fn convert_position(
        pos: &PhoenixPosition,
        subaccount_collateral_usd: f64,
        single_position: bool,
    ) -> Option<UnifiedPosition> {
        let size = pos.signed_position_size();

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
            entry_price: pos.entry_price.to_f64(),
            mark_price: 0.0,
            unrealized_pnl: pos.unrealized_pnl.to_f64(),
            cumulative_funding: pos.funding_usd(),
            leverage: pos.display_leverage(subaccount_collateral_usd, single_position),
            margin_used: pos.display_margin_usd(subaccount_collateral_usd, single_position),
            liquidation_price: {
                let liq = pos.liquidation_price.to_f64();
                if liq > 0.0 { Some(liq) } else { None }
            },
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
            .query(&[(
                "pdaIndex",
                crate::helpers::collateral::DEFAULT_TRADER_PDA_INDEX,
            )])
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
            .query(&[(
                "pdaIndex",
                crate::helpers::collateral::DEFAULT_TRADER_PDA_INDEX,
            )])
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
            .map(|trader| trader.effective_collateral_usd())
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
        let mut messages = vec![
            json!({
                "type": "subscribe",
                "subscription": { "channel": "allMids" }
            })
            .to_string(),
        ];

        for symbol in symbols {
            messages.push(
                json!({
                    "type": "subscribe",
                    "subscription": {
                        "channel": "fundingRate",
                        "symbol": normalize_phoenix_symbol(symbol)
                    }
                })
                .to_string(),
            );
        }

        messages
    }

    fn parse_ws_message(&self, raw: &str) -> Vec<WsMessage> {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
            if v.get("type").and_then(|t| t.as_str()) == Some("subscriptionConfirmed") {
                log::debug!("Phoenix WS: {raw}");
                return vec![];
            }

            match v.get("channel").and_then(|c| c.as_str()) {
                Some("error") => {
                    let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or(raw);
                    log::warn!("Phoenix WS server error: {msg}");
                    return vec![];
                }
                Some("subscriptionStatus") => {
                    log::debug!("Phoenix WS: {raw}");
                    return vec![];
                }
                Some("allMids") => {
                    if let Some(mids) = v.get("mids").and_then(|m| m.as_object()) {
                        let mut cache = self.latest_mids.lock().unwrap_or_else(|e| e.into_inner());
                        for (sym, px_val) in mids {
                            let symbol = normalize_phoenix_symbol(sym);
                            if !self.subscribed_symbols.contains(&symbol) {
                                continue;
                            }
                            if let Some(px) = px_val.as_f64() {
                                cache.insert(symbol, px);
                            }
                        }
                    }
                    return vec![];
                }
                Some("fundingRate") => {
                    let symbol = v
                        .get("symbol")
                        .and_then(|s| s.as_str())
                        .map(normalize_phoenix_symbol)
                        .unwrap_or_default();
                    if symbol.is_empty() || !self.subscribed_symbols.contains(&symbol) {
                        return vec![];
                    }
                    let funding = match v.get("funding").and_then(|f| f.as_f64()) {
                        Some(f) => f,
                        None => return vec![],
                    };
                    let mark_px = self
                        .latest_mids
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&symbol)
                        .copied()
                        .unwrap_or(0.0);
                    let funding_rate = Self::normalize_funding_to_hourly_decimal(funding);
                    return vec![WsMessage::FundingUpdate {
                        symbol,
                        mark_price: mark_px,
                        funding_rate,
                        timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    }];
                }
                _ => {}
            }
        }

        // Fallback: legacy `market` channel payloads (mark + funding in one message).
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

        let symbol = normalize_phoenix_symbol(&parsed.symbol);
        if !self.subscribed_symbols.is_empty() && !self.subscribed_symbols.contains(&symbol) {
            return vec![];
        }

        let funding_rate = Self::normalize_funding_to_hourly_decimal(funding);

        vec![WsMessage::FundingUpdate {
            symbol,
            mark_price: mark_px,
            funding_rate,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_funding_rate_with_cached_mid() {
        let exchange = PhoenixExchange::with_feed(Vec::new(), &["BTC".to_string()]);
        {
            let mut cache = exchange.latest_mids.lock().unwrap();
            cache.insert("BTC".to_string(), 73_000.0);
        }

        let raw = r#"{"channel":"fundingRate","symbol":"BTC","funding":-0.0075}"#;
        let msgs = exchange.parse_ws_message(raw);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            WsMessage::FundingUpdate {
                symbol,
                mark_price,
                funding_rate,
                ..
            } => {
                assert_eq!(symbol, "BTC");
                assert!((*mark_price - 73_000.0).abs() < f64::EPSILON);
                assert!(funding_rate.abs() > 0.0);
            }
            _ => panic!("expected FundingUpdate"),
        }
    }

    #[test]
    fn parse_market_stats_message() {
        let exchange = PhoenixExchange::new();
        let raw = r#"{
            "channel": "market",
            "symbol": "BTC",
            "markPx": 73817.0,
            "funding": -0.000625597385727166
        }"#;
        let msgs = exchange.parse_ws_message(raw);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            WsMessage::FundingUpdate { symbol, .. } => assert_eq!(symbol, "BTC"),
            _ => panic!("expected FundingUpdate"),
        }
    }

    #[test]
    fn parse_server_error_does_not_panic() {
        let exchange = PhoenixExchange::new();
        let raw = r#"{"channel":"error","code":400,"error":"missing field type"}"#;
        assert!(exchange.parse_ws_message(raw).is_empty());
    }

    #[test]
    fn normalize_funding_percent_to_hourly_decimal() {
        // Live MON WS: funding ≈ -0.00748 → UI "1h funding" −0.0075%
        let hourly = PhoenixExchange::normalize_funding_to_hourly_decimal(-0.007481669908723627);
        assert!((hourly - (-0.00007481669908723627)).abs() < 1e-12);

        // Nuke annualization: hourly * 24 * 365 * 100 ≈ −65.7%
        let annualized_pct = hourly * 24.0 * 365.0 * 100.0;
        assert!(annualized_pct > -70.0 && annualized_pct < -60.0);
    }
}
