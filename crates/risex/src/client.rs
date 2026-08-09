use std::{collections::HashMap, env, time::Duration};

use anyhow::{Context, Result, ensure};
use reqwest::Client;

use crate::types::{RiseXMarketSnapshot, RiseXMarketsEnvelope};

pub const RISEX_HTTP_URL: &str = "https://api.rise.trade";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Public, credential-free RiseX REST client.
///
/// The client deliberately exposes only market data needed by the Rust feed.
/// User account reads are implemented in the TypeScript API, which owns public
/// routes and portfolio aggregation in the current Nuketrade architecture.
#[derive(Debug, Clone)]
pub struct RiseXClient {
    http: Client,
    base_url: String,
}

impl RiseXClient {
    pub fn from_env() -> Self {
        let base_url = env::var("RISEX_HTTP_URL")
            .unwrap_or_else(|_| RISEX_HTTP_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        Self {
            http: Client::new(),
            base_url,
        }
    }

    /// Fetch current, uncached market state.
    ///
    /// RiseX otherwise permits its market response to be cached for several
    /// minutes, which is unsuitable for a live mark/funding feed. The polling
    /// frequency remains far below the documented public REST limit.
    pub async fn fetch_markets(&self) -> Result<Vec<RiseXMarketSnapshot>> {
        let response = self
            .http
            .get(format!("{}/v1/markets", self.base_url))
            .query(&[("force_refresh", "true")])
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .context("RiseX markets request failed")?
            .error_for_status()
            .context("RiseX markets request returned an unsuccessful status")?
            .json::<RiseXMarketsEnvelope>()
            .await
            .context("RiseX markets response did not match the expected schema")?;

        let mut markets = Vec::new();

        for market in response.data.markets {
            match market.into_snapshot() {
                Ok(Some(market)) => markets.push(market),
                Ok(None) => {}
                Err(err) => {
                    // One malformed market must not suppress valid market data
                    // from the remainder of the same upstream response.
                    log::warn!("Skipping invalid RiseX market: {err:#}");
                }
            }
        }

        ensure!(
            !markets.is_empty(),
            "RiseX returned no active markets covered by Nuketrade's token allowlist"
        );

        Ok(markets)
    }

    /// Build the symbol-indexed price and hourly-funding snapshot expected by
    /// Nuketrade's shared feed manager.
    pub async fn fetch_feed_snapshot(&self) -> Result<HashMap<String, (f64, f64)>> {
        Ok(self
            .fetch_markets()
            .await?
            .into_iter()
            .map(|market| {
                (
                    market.symbol,
                    (market.mark_price, market.hourly_funding_rate),
                )
            })
            .collect())
    }

    /// Load the active RiseX leverage limits used by the public feed response.
    pub async fn fetch_max_leverage_map(&self) -> Result<HashMap<String, u32>> {
        Ok(self
            .fetch_markets()
            .await?
            .into_iter()
            .map(|market| (market.symbol, market.max_leverage))
            .collect())
    }
}
