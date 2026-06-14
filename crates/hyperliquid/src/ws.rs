//! Hyperliquid WebSocket funding feed.

use crate::HyperliquidExchange;
use perp_core::{
    token_list::TOKEN_LIST,
    types::MarketFeedUpdate,
    ws::{WsConfig, run_funding_feed},
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Start the Hyperliquid funding rate feed using the generic WebSocket handler.
pub async fn start_hl_funding_feed(db_conn: Arc<PgPool>, feed_tx: mpsc::Sender<MarketFeedUpdate>) {
    let exchange = Arc::new(HyperliquidExchange::new());

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: None, // Hyperliquid handles ping/pong at protocol level
        heartbeat: perp_core::ws::WsHeartbeat::JsonMethodPing,
        subscription_delay_ms: 0,
        use_custom_ws_config: false,
    };

    let symbols: Vec<&str> = TOKEN_LIST.iter().map(|s| &**s).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbols).await;
}
