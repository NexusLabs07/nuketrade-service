use std::{collections::HashSet, sync::Arc};

use perp_core::{
    MarketFeedUpdate,
    token_list::TOKEN_LIST,
    ws::{WsConfig, WsHeartbeat, run_funding_feed},
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{BulkExchange, BulkNetwork};

/// Starts Bulk's read-only ticker feed for the reviewed public asset universe.
///
/// Flow:
///
/// 1. Fetch the current market list from `/exchangeInfo`.
/// 2. Keep active markets whose base symbol is in `TOKEN_LIST`.
/// 3. Subscribe to the Bulk ticker stream.
/// 4. Forward mark price and hourly funding into the shared feed handler.
/// 5. Let the shared handler manage reconnections, database snapshots, and
///    live-feed updates.
///
/// Market discovery is retried indefinitely because a temporary REST failure
/// during process startup must not disable Bulk until the next deployment.
pub async fn start_bulk_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
    network: BulkNetwork,
) {
    let base_exchange = BulkExchange::new(network);
    let allowed: HashSet<&str> = TOKEN_LIST.iter().copied().collect();

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,

        // Bulk sends RFC 6455 Ping frames. The shared handler responds with
        // Pong frames automatically, so a separate client heartbeat is not
        // required.
        ping_interval_secs: None,
        heartbeat: WsHeartbeat::WebSocketPing,

        // Bulk accepts all ticker subscriptions in one batch.
        subscription_delay_ms: 0,

        stale_threshold_secs: 60,
        use_custom_ws_config: false,
    };

    // Keep discovery inside the background task so a transient metadata
    // failure cannot permanently remove Bulk from the running process.
    let (exchange_symbols, shared_markets) = loop {
        match base_exchange.fetch_market_info().await {
            Ok(markets) => {
                let mut exchange_symbols = Vec::new();
                let mut shared_markets = Vec::new();

                for market in markets {
                    if !market.is_active || !allowed.contains(market.symbol.as_str()) {
                        continue;
                    }

                    // Bulk subscriptions use the full market name, while the
                    // shared feed stores the normalized base symbol.
                    exchange_symbols.push(format!("{}-USD", market.symbol));
                    shared_markets.push(market);
                }

                exchange_symbols.sort_unstable();
                exchange_symbols.dedup();

                if !exchange_symbols.is_empty() {
                    break (exchange_symbols, shared_markets);
                }

                log::warn!("Bulk: metadata contained no active allowlisted markets");
            }

            Err(error) => {
                log::warn!("Bulk: market discovery failed: {error}");
            }
        }

        log::info!(
            "Bulk: retrying market discovery in {} seconds",
            config.reconnect_delay_secs
        );

        tokio::time::sleep(config.reconnect_delay()).await;
    };

    log::info!(
        "Bulk: starting funding feed for {} active allowlisted markets",
        exchange_symbols.len()
    );

    let exchange = Arc::new(BulkExchange::with_markets_for_network(
        network,
        shared_markets,
    ));
    let symbol_refs: Vec<&str> = exchange_symbols.iter().map(String::as_str).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbol_refs).await;
}
