use std::collections::HashMap;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use perp_core::token_list::TOKEN_LIST;

/// Minimal envelope returned by `GET /v1/markets`.
///
/// RiseX is actively evolving this response. Only fields required by the
/// read-only feed are deserialized so additive upstream changes remain
/// backwards compatible.
#[derive(Debug, Deserialize)]
pub(crate) struct RiseXMarketsEnvelope {
    pub data: RiseXMarketsData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RiseXMarketsData {
    pub markets: Vec<RiseXWireMarket>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RiseXWireMarket {
    pub market_id: String,
    pub config: RiseXWireMarketConfig,
    pub mark_price: String,
    pub current_funding_rate: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RiseXWireMarketConfig {
    pub name: String,
    pub max_leverage: String,
    pub unlocked: bool,
}

/// Validated market metadata consumed by Nuketrade's shared feed pipeline.
#[derive(Debug, Clone)]
pub struct RiseXMarketSnapshot {
    /// RiseX numeric market identifier used by its WebSocket channels.
    pub market_id: u64,

    /// Nuketrade-normalized base symbol, such as `BTC`.
    pub symbol: String,

    /// Human-readable mark price.
    pub mark_price: f64,

    /// RiseX native one-hour funding rate.
    ///
    /// This must not be replaced by the separate `funding_rate_8h` field.
    pub hourly_funding_rate: f64,

    pub max_leverage: u32,
}

/// Generic RiseX WebSocket envelope.
///
/// Subscription acknowledgements and oracle updates share the same outer
/// envelope, so all control fields remain optional.
#[derive(Debug, Deserialize)]
pub(crate) struct RiseXSocketEnvelope {
    pub channel: Option<String>,

    #[serde(rename = "type")]
    pub message_type: Option<String>,

    pub status: Option<String>,
    pub message: Option<String>,

    #[serde(default)]
    pub data: RiseXOracleData,
}

/// Oracle-channel payload.
///
/// Prices are keyed by RiseX market ID. Values use 18-decimal fixed-point
/// encoding rather than human-readable decimal strings.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RiseXOracleData {
    #[serde(default)]
    pub prices: HashMap<String, RiseXOraclePrice>,
}

/// Price values published by the RiseX oracle WebSocket channel.
#[derive(Debug, Deserialize)]
pub(crate) struct RiseXOraclePrice {
    pub mark_price: String,
}

impl RiseXWireMarket {
    /// Convert an upstream market into Nuketrade's validated representation.
    ///
    /// Locked, inactive, and non-allowlisted markets are intentionally omitted.
    /// Invalid numeric values fail only the affected market; the client logs and
    /// skips it without suppressing valid markets from the same response.
    pub(crate) fn into_snapshot(self) -> Result<Option<RiseXMarketSnapshot>> {
        if !self.active || !self.config.unlocked {
            return Ok(None);
        }

        let symbol = self
            .config
            .name
            .split_once('/')
            .map_or(self.config.name.as_str(), |(base, _)| base)
            .trim()
            .to_ascii_uppercase();

        if !TOKEN_LIST.contains(&symbol.as_str()) {
            return Ok(None);
        }

        let market_id = self
            .market_id
            .parse::<u64>()
            .with_context(|| format!("invalid market ID for RiseX {symbol}"))?;

        let mark_price = self
            .mark_price
            .parse::<f64>()
            .with_context(|| format!("invalid mark price for RiseX {symbol}"))?;

        let hourly_funding_rate = self
            .current_funding_rate
            .parse::<f64>()
            .with_context(|| format!("invalid current funding rate for RiseX {symbol}"))?;

        let max_leverage = self
            .config
            .max_leverage
            .parse::<u32>()
            .with_context(|| format!("invalid max leverage for RiseX {symbol}"))?;

        ensure!(market_id > 0, "zero market ID for RiseX {symbol}");

        ensure!(
            mark_price.is_finite() && mark_price > 0.0,
            "non-positive or non-finite mark price for RiseX {symbol}"
        );

        ensure!(
            hourly_funding_rate.is_finite(),
            "non-finite funding rate for RiseX {symbol}"
        );

        ensure!(max_leverage > 0, "zero max leverage for RiseX {symbol}");

        Ok(Some(RiseXMarketSnapshot {
            market_id,
            symbol,
            mark_price,
            hourly_funding_rate,
            max_leverage,
        }))
    }
}
