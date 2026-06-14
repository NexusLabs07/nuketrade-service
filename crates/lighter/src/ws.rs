//! Lighter WebSocket funding feed.

use std::{collections::HashSet, sync::Arc};

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::LighterExchange;
use perp_core::{
    token_list::TOKEN_LIST,
    types::MarketFeedUpdate,
    ws::{WsConfig, run_funding_feed},
};

/// Start the Lighter funding rate feed using the generic WebSocket handler.
pub async fn start_lighter_funding_feed(
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
) {
    let discovery = LighterExchange::new();

    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: Some(60),
        heartbeat: perp_core::ws::WsHeartbeat::JsonMethodPing,
        subscription_delay_ms: 0,
        use_custom_ws_config: true,
    };

    let allowed_symbols: HashSet<&str> = TOKEN_LIST.iter().copied().collect();

    let lighter_markets = loop {
        match discovery.fetch_active_perp_markets().await {
            Ok(markets) => {
                let filtered: Vec<_> = markets
                    .into_iter()
                    .filter(|market| allowed_symbols.contains(market.symbol.as_str()))
                    .collect();

                if !filtered.is_empty() {
                    break filtered;
                }

                log::warn!(
                    "Lighter: no TOKEN_LIST perp markets discovered, retrying in {} seconds",
                    config.reconnect_delay_secs
                );
            }
            Err(err) => {
                log::error!("Lighter: failed to fetch market metadata before WS start: {err}");
            }
        }

        tokio::time::sleep(config.reconnect_delay()).await;
    };

    let missing_symbols: Vec<&str> = TOKEN_LIST
        .iter()
        .copied()
        .filter(|symbol| {
            !lighter_markets
                .iter()
                .any(|market| market.symbol.as_str() == *symbol)
        })
        .collect();

    if !missing_symbols.is_empty() {
        log::warn!(
            "Lighter: TOKEN_LIST symbols not active on Lighter or not returned by metadata endpoint: {}",
            missing_symbols.join(", ")
        );
    }

    let symbols: Vec<String> = lighter_markets
        .iter()
        .map(|market| market.symbol.clone())
        .collect();

    log::info!(
        "Lighter: starting funding feed for {} TOKEN_LIST markets",
        symbols.len()
    );

    let exchange = Arc::new(LighterExchange::with_markets(lighter_markets));
    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbol_refs).await;
}
