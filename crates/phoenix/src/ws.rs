use std::{collections::HashSet, sync::Arc};

use crate::PhoenixExchange;
use perp_core::{
    token_list::TOKEN_LIST,
    types::MarketFeedUpdate,
    ws::{WsConfig, WsHeartbeat, run_funding_feed},
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

    let allowed: HashSet<&str> = TOKEN_LIST.iter().copied().collect();
    let phoenix_markets: Vec<_> = markets
        .into_iter()
        .filter(|m| m.is_active && allowed.contains(m.symbol.as_str()))
        .collect();

    if phoenix_markets.is_empty() {
        log::error!("Phoenix: no active TOKEN_LIST markets from metadata; funding feed not started");
        return;
    }

    let missing: Vec<&str> = TOKEN_LIST
        .iter()
        .copied()
        .filter(|symbol| !phoenix_markets.iter().any(|m| m.symbol == *symbol))
        .collect();
    if !missing.is_empty() {
        log::info!(
            "Phoenix: TOKEN_LIST symbols not listed on Phoenix: {}",
            missing.join(", ")
        );
    }

    let symbols: Vec<String> = phoenix_markets
        .iter()
        .map(|m| m.symbol.clone())
        .collect();
    log::info!(
        "Phoenix: starting funding feed for {} markets",
        symbols.len()
    );

    let exchange = Arc::new(PhoenixExchange::with_feed(
        phoenix_markets,
        &symbols,
    ));
    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();

    // Phoenix WS: all client JSON must include `"type"` — do not send `{"method":"ping"}`.
    // Use WebSocket Ping frames for keepalive (see https://docs.phoenix.trade/api/websocket).
    let config = WsConfig {
        max_retries: 3,
        reconnect_delay_secs: 5,
        db_write_interval_secs: 30 * 60,
        state_update_interval_secs: 5,
        stale_threshold_secs: 60,
        ping_interval_secs: Some(30),
        heartbeat: WsHeartbeat::WebSocketPing,
        subscription_delay_ms: 100,
        use_custom_ws_config: false,
    };

    run_funding_feed(exchange, db_conn, feed_tx, config, &symbol_refs).await;
}
