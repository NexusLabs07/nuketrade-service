//! Pacifica WebSocket funding feed.

use crate::PacificaExchange;
use perp_core::{
    types::MarketFeedUpdate,
    ws::{WsConfig, run_funding_feed},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Pacifica prices WebSocket message format.
#[derive(Debug, Serialize, Deserialize)]
pub struct PricesMessage {
    pub channel: String,
    pub data: Vec<PriceData>,
}

/// Individual price data in a Pacifica prices message.
#[derive(Debug, Serialize, Deserialize)]
pub struct PriceData {
    pub funding: String,
    pub mark: String,
    pub mid: String,
    pub next_funding: String,
    pub open_interest: String,
    pub oracle: String,
    pub symbol: String,
    pub timestamp: i64,
    pub volume_24h: String,
    pub yesterday_price: String,
}

/// Start the Pacifica funding rate feed using the generic WebSocket handler.
pub async fn start_pacifica_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let exchange = Arc::new(PacificaExchange::new());

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: Some(30), // Pacifica needs periodic pings
        heartbeat: perp_core::ws::WsHeartbeat::JsonMethodPing,
        subscription_delay_ms: 0,
        use_custom_ws_config: true,
    };

    // Pacifica's prices subscription ignores per-symbol inputs and streams all markets.
    let symbols: [&str; 0] = [];

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbols).await;
}
