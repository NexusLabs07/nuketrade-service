use std::{collections::HashMap, env, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use db::funding::{FundingRate, insert_funding_rates};
use futures_util::{SinkExt, StreamExt};
use perp_core::{MarketFeedUpdate, exchange::PerpetualExchange};
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::mpsc,
    time::{Instant, MissedTickBehavior},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{
    RiseXClient,
    types::{RiseXMarketSnapshot, RiseXSocketEnvelope},
};

const DEFAULT_RISEX_WS_URL: &str = "wss://api.rise.trade/ws/";

/// REST is retained for funding rates, market metadata, and WebSocket fallback.
const DEFAULT_REST_REFRESH_SECONDS: u64 = 30;
const MINIMUM_REST_REFRESH_SECONDS: u64 = 5;
const MAXIMUM_REST_REFRESH_SECONDS: u64 = 300;

const DEFAULT_RECONNECT_SECONDS: u64 = 5;
const DEFAULT_STALE_SECONDS: u64 = 90;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);
const DATABASE_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// RiseX oracle values use 18-decimal fixed-point encoding.
const WAD_SCALE: f64 = 1_000_000_000_000_000_000.0;

/// In-memory state joining REST market metadata with WebSocket oracle updates.
#[derive(Debug, Default)]
struct RiseXFeedState {
    /// RiseX market ID to Nuketrade symbol.
    symbol_by_market_id: HashMap<u64, String>,

    /// Nuketrade symbol to current `(mark price, hourly funding rate)`.
    snapshot: HashMap<String, (f64, f64)>,
}

impl RiseXFeedState {
    /// Merge a validated REST snapshot into the current state.
    ///
    /// Existing WebSocket mark prices are preserved while funding rates and
    /// market-ID mappings are refreshed. Newly listed markets start with their
    /// REST mark price until the next oracle update arrives.
    fn merge_rest_markets(&mut self, markets: Vec<RiseXMarketSnapshot>) {
        let mut next_symbol_by_market_id = HashMap::with_capacity(markets.len());
        let mut next_snapshot = HashMap::with_capacity(markets.len());

        for market in markets {
            next_symbol_by_market_id.insert(market.market_id, market.symbol.clone());

            let mark_price = self
                .snapshot
                .get(&market.symbol)
                .map_or(market.mark_price, |(current_mark, _)| *current_mark);

            next_snapshot.insert(market.symbol, (mark_price, market.hourly_funding_rate));
        }

        self.symbol_by_market_id = next_symbol_by_market_id;
        self.snapshot = next_snapshot;
    }

    /// Apply one oracle message and return only changed market values.
    fn apply_oracle_prices(
        &mut self,
        envelope: RiseXSocketEnvelope,
    ) -> HashMap<String, (f64, f64)> {
        let mut changed = HashMap::new();

        for (market_id, oracle_price) in envelope.data.prices {
            let Ok(market_id) = market_id.parse::<u64>() else {
                log::warn!("RiseX oracle update contained an invalid market ID: {market_id}");
                continue;
            };

            let Some(symbol) = self.symbol_by_market_id.get(&market_id).cloned() else {
                // Unknown or newly listed markets are ignored until REST refreshes
                // the authoritative market-ID mapping.
                continue;
            };

            let Ok(encoded_mark_price) = oracle_price.mark_price.parse::<f64>() else {
                log::warn!("RiseX oracle returned an invalid mark price for {symbol}");
                continue;
            };

            let mark_price = encoded_mark_price / WAD_SCALE;

            if !mark_price.is_finite() || mark_price <= 0.0 {
                log::warn!("RiseX oracle returned a non-positive mark price for {symbol}");
                continue;
            }

            let Some((current_mark, funding_rate)) = self.snapshot.get_mut(&symbol) else {
                continue;
            };

            *current_mark = mark_price;
            changed.insert(symbol, (mark_price, *funding_rate));
        }

        changed
    }
}

/// Result of parsing one WebSocket text frame.
enum OracleFrame {
    /// The server accepted the subscription.
    Subscribed,

    /// A validated oracle update has been applied.
    Update(HashMap<String, (f64, f64)>),

    /// The frame is valid but unrelated to the oracle feed.
    Ignore,

    /// The server explicitly rejected the request.
    Rejected(String),
}

fn configured_duration(
    variable: &str,
    default_seconds: u64,
    minimum_seconds: u64,
    maximum_seconds: u64,
) -> Duration {
    let seconds = match env::var(variable) {
        Ok(value) => match value.parse::<u64>() {
            Ok(value) => value.clamp(minimum_seconds, maximum_seconds),
            Err(error) => {
                log::warn!("Ignoring invalid {variable} value: {error}");
                default_seconds
            }
        },
        Err(_) => default_seconds,
    };

    Duration::from_secs(seconds)
}

fn rest_refresh_interval() -> Duration {
    // Preserve the existing variable name so current deployments do not require
    // an immediate configuration migration.
    configured_duration(
        "RISEX_POLL_INTERVAL_SECONDS",
        DEFAULT_REST_REFRESH_SECONDS,
        MINIMUM_REST_REFRESH_SECONDS,
        MAXIMUM_REST_REFRESH_SECONDS,
    )
}

fn reconnect_interval() -> Duration {
    configured_duration(
        "RISEX_WS_RECONNECT_SECONDS",
        DEFAULT_RECONNECT_SECONDS,
        1,
        60,
    )
}

fn stale_interval() -> Duration {
    configured_duration("RISEX_WS_STALE_SECONDS", DEFAULT_STALE_SECONDS, 30, 600)
}

fn websocket_url() -> String {
    env::var("RISEX_WS_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_RISEX_WS_URL.to_string())
}

fn funding_rows(snapshot: &HashMap<String, (f64, f64)>) -> Vec<FundingRate> {
    let timestamp = Utc::now().naive_utc();

    snapshot
        .iter()
        .map(|(symbol, (mark_price, hourly_funding_rate))| FundingRate {
            id: Uuid::new_v4(),
            platform: PerpetualExchange::RiseX.to_string(),
            symbol: symbol.clone(),
            rate: *hourly_funding_rate,
            mark_px: *mark_price,
            timestamp,
        })
        .collect()
}

/// Publish the current RiseX state through Nuketrade's shared feed manager.
async fn publish_snapshot(
    feed_tx: &mpsc::Sender<MarketFeedUpdate>,
    data: HashMap<String, (f64, f64)>,
) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    feed_tx
        .send(MarketFeedUpdate {
            exchange: PerpetualExchange::RiseX,
            data,
        })
        .await
        .context("RiseX feed manager receiver dropped")
}

/// Refresh market mappings and funding rates from the public REST API.
async fn refresh_from_rest(
    client: &RiseXClient,
    state: &mut RiseXFeedState,
    feed_tx: &mpsc::Sender<MarketFeedUpdate>,
) -> Result<()> {
    let markets = client.fetch_markets().await?;
    state.merge_rest_markets(markets);

    publish_snapshot(feed_tx, state.snapshot.clone()).await
}

/// Persist the latest complete RiseX state on the shared thirty-minute cadence.
fn persist_if_due(
    state: &RiseXFeedState,
    db_conn: &Arc<PgPool>,
    last_database_attempt: &mut Option<Instant>,
) {
    if state.snapshot.is_empty() {
        return;
    }

    let write_due =
        last_database_attempt.is_none_or(|attempt| attempt.elapsed() >= DATABASE_SNAPSHOT_INTERVAL);

    if !write_due {
        return;
    }

    *last_database_attempt = Some(Instant::now());

    let rows = funding_rows(&state.snapshot);
    let db = db_conn.clone();

    tokio::spawn(async move {
        if let Err(error) = insert_funding_rates(db, rows).await {
            log::warn!("RiseX funding-rate database insert failed: {error:#}");
        }
    });
}

/// Parse and apply one WebSocket text frame.
fn parse_oracle_frame(state: &mut RiseXFeedState, raw: &str) -> Result<OracleFrame> {
    let envelope = serde_json::from_str::<RiseXSocketEnvelope>(raw)
        .context("RiseX WebSocket returned invalid JSON")?;

    if envelope.status.as_deref() == Some("error") || envelope.channel.as_deref() == Some("error") {
        return Ok(OracleFrame::Rejected(envelope.message.unwrap_or_else(
            || "RiseX rejected the WebSocket request".to_string(),
        )));
    }

    if envelope.message_type.as_deref() == Some("subscribed")
        && envelope.status.as_deref() == Some("success")
        && envelope.channel.as_deref() == Some("oracle")
    {
        return Ok(OracleFrame::Subscribed);
    }

    if envelope.channel.as_deref() != Some("oracle")
        || envelope.message_type.as_deref() != Some("update")
    {
        return Ok(OracleFrame::Ignore);
    }

    Ok(OracleFrame::Update(state.apply_oracle_prices(envelope)))
}

/// Run one RiseX WebSocket connection until it closes or becomes stale.
async fn run_websocket_session(
    client: &RiseXClient,
    state: &mut RiseXFeedState,
    db_conn: &Arc<PgPool>,
    feed_tx: &mpsc::Sender<MarketFeedUpdate>,
    last_database_attempt: &mut Option<Instant>,
) -> Result<()> {
    let url = websocket_url();

    let (socket, _) = tokio::time::timeout(CONNECTION_TIMEOUT, connect_async(&url))
        .await
        .context("RiseX WebSocket connection timed out")?
        .context("RiseX WebSocket connection failed")?;

    let (mut write, mut read) = socket.split();

    let subscription = json!({
        "method": "subscribe",
        "params": {
            "channel": "oracle"
        }
    })
    .to_string();

    write
        .send(Message::Text(subscription.into()))
        .await
        .context("failed to subscribe to the RiseX oracle channel")?;

    log::info!("RiseX WebSocket connected; oracle subscription sent");

    let mut rest_tick = tokio::time::interval(rest_refresh_interval());
    rest_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // The REST state was refreshed immediately before opening this session.
    rest_tick.tick().await;

    let mut health_tick = tokio::time::interval(Duration::from_secs(10));
    health_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    health_tick.tick().await;

    let mut last_oracle_update = Instant::now();

    loop {
        tokio::select! {
            incoming = read.next() => {
                let message = match incoming {
                    Some(Ok(message)) => message,
                    Some(Err(error)) => {
                        return Err(error).context("RiseX WebSocket read failed");
                    }
                    None => bail!("RiseX WebSocket stream ended"),
                };

                match message {
                    Message::Text(text) => {
                        match parse_oracle_frame(state, text.as_ref()) {
                            Ok(OracleFrame::Subscribed) => {
                                log::info!("RiseX oracle WebSocket subscription confirmed");
                            }
                            Ok(OracleFrame::Update(changed)) => {
                                if changed.is_empty() {
                                    continue;
                                }

                                last_oracle_update = Instant::now();
                                publish_snapshot(feed_tx, changed).await?;
                                persist_if_due(state, db_conn, last_database_attempt);
                            }
                            Ok(OracleFrame::Ignore) => {}
                            Ok(OracleFrame::Rejected(reason)) => {
                                bail!("RiseX rejected the oracle subscription: {reason}");
                            }
                            Err(error) => {
                                // One malformed frame must not discard the last good
                                // state or unnecessarily restart a healthy connection.
                                log::warn!("Ignoring invalid RiseX WebSocket frame: {error:#}");
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        write
                            .send(Message::Pong(payload))
                            .await
                            .context("failed to answer RiseX WebSocket ping")?;
                    }
                    Message::Pong(_) => {}
                    Message::Close(frame) => {
                        bail!("RiseX WebSocket closed: {frame:?}");
                    }
                    Message::Binary(_) | Message::Frame(_) => {}
                }
            }

            _ = rest_tick.tick() => {
                match refresh_from_rest(client, state, feed_tx).await {
                    Ok(()) => {
                        persist_if_due(state, db_conn, last_database_attempt);
                    }
                    Err(error) => {
                        // Keep serving the last good WebSocket/REST state during
                        // temporary upstream REST failures.
                        log::warn!(
                            "RiseX REST refresh failed; retaining last good state: {error:#}"
                        );
                    }
                }
            }

            _ = health_tick.tick() => {
                if last_oracle_update.elapsed() >= stale_interval() {
                    bail!(
                        "RiseX oracle WebSocket produced no updates for {} seconds",
                        last_oracle_update.elapsed().as_secs()
                    );
                }
            }
        }
    }
}

/// Start the read-only RiseX market feed.
///
/// Mark prices come from the native public oracle WebSocket. REST remains the
/// authoritative source for market-ID mappings and hourly funding rates, and
/// also keeps the feed alive while the socket reconnects.
pub async fn start_risex_funding_feed(
    client: RiseXClient,
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let mut state = RiseXFeedState::default();
    let mut last_database_attempt: Option<Instant> = None;

    loop {
        // Refresh REST before every connection attempt. This provides an
        // immediate snapshot and updates market mappings after listings change.
        match refresh_from_rest(&client, &mut state, &feed_tx).await {
            Ok(()) => {
                persist_if_due(&state, &db_conn, &mut last_database_attempt);
            }
            Err(error) => {
                log::warn!("RiseX REST bootstrap failed; retaining last good state: {error:#}");
            }
        }

        if feed_tx.is_closed() {
            log::info!("RiseX feed manager receiver dropped; stopping feed");
            return;
        }

        if let Err(error) = run_websocket_session(
            &client,
            &mut state,
            &db_conn,
            &feed_tx,
            &mut last_database_attempt,
        )
        .await
        {
            if feed_tx.is_closed() {
                log::info!("RiseX feed manager receiver dropped; stopping feed");
                return;
            }

            log::warn!(
                "RiseX WebSocket session ended; reconnecting in {} seconds: {error:#}",
                reconnect_interval().as_secs()
            );
        }

        tokio::time::sleep(reconnect_interval()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Protect against applying an 18-decimal encoded price directly or
    /// associating an oracle market ID with the wrong Nuketrade symbol.
    #[test]
    fn oracle_update_scales_price_and_uses_market_id_mapping() {
        let mut state = RiseXFeedState {
            symbol_by_market_id: HashMap::from([(1, "BTC".to_string())]),
            snapshot: HashMap::from([("BTC".to_string(), (65_000.0, -0.000_001_5))]),
        };

        let raw = r#"{
            "channel": "oracle",
            "type": "update",
            "market_id": null,
            "data": {
                "prices": {
                    "1": {
                        "index_price": "65155336306733981000000",
                        "mark_price": "65143560582178186025787"
                    }
                },
                "timestamp": 1786343715
            }
        }"#;

        let frame = parse_oracle_frame(&mut state, raw).expect("frame should parse");

        let OracleFrame::Update(changed) = frame else {
            panic!("expected an oracle update");
        };

        let (mark_price, funding_rate) =
            changed.get("BTC").copied().expect("BTC should be updated");

        assert!((mark_price - 65_143.560_582_178_19).abs() < 0.000_001);
        assert_eq!(funding_rate, -0.000_001_5);
    }
}
