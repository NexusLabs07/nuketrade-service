use std::sync::Arc;

use crate::PhoenixExchange;
use perp_core::{
    token_list::TOKEN_LIST,
    types::MarketFeedUpdate,
    ws::{WsConfig, run_funding_feed},
};
use sqlx::PgPool;
use tokio::sync::mpsc;

pub async fn start_phoenix_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let base_exchange = PhoenixExchange::new();

    let markets = match base_exchange.fetch_market_info().await {
        Ok(markets) => markets,
        Err(err) => {
            log::warn!(
                "Phoenix: failed to fetch market metadata, continuing with empty map: {err}"
            );
            Vec::new()
        }
    };

    let exchange = Arc::new(PhoenixExchange::with_markets(markets));

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: Some(30),
        use_custom_ws_config: false,
    };

    let symbols: Vec<&str> = TOKEN_LIST.iter().map(|s| &**s).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbols).await;
}
