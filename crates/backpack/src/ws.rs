use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

use perp_core::{MarketFeedUpdate, WsConfig, run_funding_feed, token_list::TOKEN_LIST};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{BackpackExchange, helpers::markets::is_allowed_backpack_symbol};

/// this starts the backpack public funding feed
///
/// flow is something like:
/// - Discover active PERP symbols from REST
/// - Keep only markets whose canonical symbol exists in TOKEN_LIST
/// - Subscribe to `markPrice.<symbol>` for each filtered market
/// - shared core websocket handler manage reconnects and ping/pong
pub async fn start_backpack_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let exchange = Arc::new(BackpackExchange::new());

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: None,
        use_custom_ws_config: false,
    };

    let allowed_symbols: HashSet<&str> = TOKEN_LIST.iter().copied().collect();

    let backpack_symbols = loop {
        match exchange.fetch_active_markets().await {
            Ok(markets) => {
                let symbols: Vec<String> = markets
                    .into_iter()
                    .filter(|market| is_allowed_backpack_symbol(&market.symbol, &allowed_symbols))
                    .map(|market| market.symbol)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();

                if !symbols.is_empty() {
                    break symbols;
                }

                log::warn!(
                    "Backpack: no TOKEN_LIST PERP markets found, retrying in {} seconds",
                    config.reconnect_delay_secs
                );
            }
            Err(err) => {
                log::error!("Backpack: failed to fetch PERP markets before WS start: {err}");
            }
        };

        tokio::time::sleep(config.reconnect_delay()).await;
    };

    log::info!(
        "Backpack: starting funding feed for {} TOKEN_LIST markets",
        backpack_symbols.len()
    );

    let symbol_refs: Vec<&str> = backpack_symbols.iter().map(String::as_str).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbol_refs).await;
}
