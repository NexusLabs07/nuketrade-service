use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use perp_core::token_list::TOKEN_LIST;

/// minimal envelope returned by `GET /v1/markets`
///
/// RiseX is actively evolving this response. We deserialize only fields needed
/// by Nuketrade so additive upstream changes remain backwards compatible
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

/// validated market values consumed by Nuketrade's shared feed pipeline
#[derive(Debug, Clone)]
pub struct RiseXMarketSnapshot {
    pub symbol: String,
    pub mark_price: f64,

    /// native one hour funding rate
    ///
    /// This must not be replaced by RiseX's separate `funding_rate_8h` field.
    pub hourly_funding_rate: f64,

    pub max_leverage: u32,
}

impl RiseXWireMarket {
    /// Convert an upstream market into Nuketrade's validated representation.
    ///
    /// Locked, inactive, and non-allowlisted markets are intentionally omitted.
    /// Invalid numeric values fail this market only; the client logs and skips
    /// it without dropping otherwise valid markets from the same response.
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
            symbol,
            mark_price,
            hourly_funding_rate,
            max_leverage,
        }))
    }
}
