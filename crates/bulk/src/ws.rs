use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

use perp_core::{MarketFeedUpdate, WsConfig, run_funding_feed, token_list::TOKEN_LIST};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::BulkExchange;

/// Starts the Bulk public funding feed.
///
/// Flow:
/// - discover active markets from REST `/exchangeInfo`
/// - keep only TOKEN_LIST assets
/// - subscribe to `ticker` for each remaining symbol
/// - use shared core websocket runner for reconnect/db writes/live fanout
pub async fn start_bulk_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let exchange = Arc::new(BulkExchange::new());

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

    let bulk_symbols = loop {
        match exchange.fetch_active_markets().await {
            Ok(markets) => {
                let symbols: Vec<String> = markets
                    .into_iter()
                    .filter(|market| {
                        crate::helpers::markets::canonical_symbol_from_bulk_symbol(&market.symbol)
                            .as_deref()
                            .is_some_and(|symbol| allowed_symbols.contains(symbol))
                    })
                    .map(|market| market.symbol)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();

                if !symbols.is_empty() {
                    break symbols;
                }

                log::warn!(
                    "Bulk: no TOKEN_LIST markets found, retrying in {} seconds",
                    config.reconnect_delay_secs
                );
            }
            Err(err) => {
                log::error!("Bulk: failed to fetch exchange info before WS start: {err}");
            }
        };

        tokio::time::sleep(config.reconnect_delay()).await;
    };

    log::info!(
        "Bulk: starting funding feed for {} TOKEN_LIST markets",
        bulk_symbols.len()
    );

    let symbol_refs: Vec<&str> = bulk_symbols.iter().map(String::as_str).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbol_refs).await;
}
