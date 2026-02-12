//! Generic WebSocket handler for exchange funding feeds.

use crate::{Exchange, MarketFeedUpdate, WsMessage, exchange::PerpetualExchange};
use chrono::Utc;
use db::funding::{FundingRate, insert_funding_rates};
use futures_util::{SinkExt, StreamExt};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    time::{Instant, interval, interval_at},
};
use tokio_tungstenite::{
    connect_async, connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};
use uuid::Uuid;

use super::WsConfig;

/// Run the funding feed for a given exchange.
///
/// This function handles:
/// - WebSocket connection with automatic reconnection
/// - Subscription to market data channels
/// - State updates to the live market feed
/// - Periodic database writes for funding snapshots
/// - Ping/pong keepalive handling
///
/// # Arguments
/// * `exchange` - The exchange implementation
/// * `db_conn` - Database connection pool
/// * `live_market_feed` - Shared state for live market data
/// * `config` - WebSocket configuration
/// * `symbols` - List of symbols to subscribe to
pub async fn run_funding_feed<E: Exchange + 'static>(
    exchange: Arc<E>,
    db_conn: Arc<PgPool>,
    feed_tx: mpsc::Sender<MarketFeedUpdate>,
    config: WsConfig,
    symbols: &[&str],
) {
    let perpetual_exchange = exchange.exchange();
    let exchange_name = exchange.name();

    // Track state across reconnections: symbol -> (mark_price, funding_rate, timestamp_ms)
    let mut last_snapshot: HashMap<String, (f64, f64, i64)> = HashMap::new();

    // Outer reconnection loop
    loop {
        let mut retry_count = 0;

        // Connection loop with retries
        let ws_stream = loop {
            let connect_result = if config.use_custom_ws_config {
                let ws_config = WebSocketConfig::default();
                connect_async_with_config(exchange.ws_url(), Some(ws_config), true).await
            } else {
                connect_async(exchange.ws_url()).await
            };

            match connect_result {
                Ok((ws_stream, _)) => break ws_stream,
                Err(e) => {
                    retry_count += 1;
                    log::error!(
                        "Error connecting to {} WS (attempt {}/{}): {}",
                        exchange_name,
                        retry_count,
                        config.max_retries,
                        e
                    );

                    if retry_count >= config.max_retries {
                        log::error!(
                            "{}: Max retries reached. Waiting {} seconds before retry cycle...",
                            exchange_name,
                            config.reconnect_delay_secs * 2
                        );
                        tokio::time::sleep(Duration::from_secs(config.reconnect_delay_secs * 2))
                            .await;
                        retry_count = 0;
                        continue;
                    }

                    log::info!(
                        "{}: Retrying in {} seconds...",
                        exchange_name,
                        config.reconnect_delay_secs
                    );
                    tokio::time::sleep(config.reconnect_delay()).await;
                }
            }
        };

        log::info!("Connected to {exchange_name} WS");

        // Set up intervals - align DB writes to wall-clock time so all exchanges write simultaneously
        let aligned_start = config.next_aligned_db_write();
        log::info!(
            "{}: Next aligned DB write in {} seconds",
            exchange_name,
            aligned_start.duration_since(Instant::now()).as_secs()
        );
        let mut db_tick = interval_at(aligned_start, config.db_write_interval());
        let mut state_tick = interval(config.state_update_interval());
        let mut last_update = Instant::now();

        // Optional ping interval
        let ping_interval = config.ping_interval();
        let mut ping_tick = ping_interval
            .map(|duration| interval_at(Instant::now() + Duration::from_secs(30), duration));

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to channels
        let subscribe_messages = exchange.build_subscribe_message(symbols);
        for msg in subscribe_messages {
            match write.send(Message::Text(msg.clone().into())).await {
                Ok(_) => log::info!("{exchange_name}: Sent subscription message"),
                Err(e) => {
                    log::error!("{exchange_name}: Error sending subscription: {e}");
                    tokio::time::sleep(config.reconnect_delay()).await;
                    continue;
                }
            }
        }

        // Message processing loop
        let should_reconnect = loop {
            tokio::select! {
                // Handle incoming messages
                msg_opt = read.next() => {
                    match msg_opt {
                        Some(Ok(msg)) => {
                            let (keep_alive, updates) = handle_message(
                                msg,
                                &mut write,
                                &*exchange,
                                symbols,
                            ).await;

                            if !updates.is_empty() {
                                last_update = Instant::now();
                                for (symbol, mark_px, funding, ts) in updates {
                                    last_snapshot.insert(symbol, (mark_px, funding, ts));
                                }
                            }

                            if !keep_alive {
                                log::warn!("{exchange_name} WS connection closed. Reconnecting...");
                                break true;
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("{exchange_name} WS read error: {e}. Reconnecting...");
                            break true;
                        }
                        None => {
                            log::warn!("{exchange_name} WS stream ended. Reconnecting...");
                            break true;
                        }
                    }
                }

                // Update live state
                _ = state_tick.tick() => {
                    send_live_update(&feed_tx, &perpetual_exchange, &last_snapshot).await;
                }

                // Write to database (aligned across all exchanges)
                _ = db_tick.tick() => {
                    let now = Utc::now();
                    log::info!(
                        "{}: DB write triggered at {} (aligned)",
                        exchange_name,
                        now.format("%H:%M:%S")
                    );

                    if last_update.elapsed() > config.stale_threshold() {
                        log::warn!("{exchange_name}: Skipping DB write - data is stale");
                        continue;
                    }

                    write_funding_to_database(&db_conn, &perpetual_exchange, &last_snapshot).await;
                }

                // Handle optional ping
                _ = async {
                    if let Some(ref mut tick) = ping_tick {
                        tick.tick().await
                    } else {
                        // If no ping configured, wait forever (effectively disabling this branch)
                        std::future::pending::<tokio::time::Instant>().await
                    }
                } => {
                    let ping_msg = serde_json::json!({"method": "ping"}).to_string();
                    if let Err(e) = write.send(Message::Text(ping_msg.into())).await {
                        log::error!("{exchange_name}: Failed to send ping: {e}. Reconnecting...");
                        break true;
                    } else {
                        log::info!("{exchange_name}: Sent heartbeat ping");
                    }
                }
            }
        };

        if should_reconnect {
            log::info!(
                "{}: Waiting {} seconds before reconnecting...",
                exchange_name,
                config.reconnect_delay_secs
            );
            tokio::time::sleep(config.reconnect_delay()).await;
        }
    }
}

/// Handle a single WebSocket message.
///
/// Returns (keep_alive, updates) where:
/// - keep_alive: false if connection should be closed
/// - updates: list of (symbol, mark_price, funding_rate, timestamp_ms) tuples
async fn handle_message<E: Exchange>(
    msg: Message,
    write: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    exchange: &E,
    symbols: &[&str],
) -> (bool, Vec<(String, f64, f64, i64)>) {
    match msg {
        Message::Ping(p) => {
            log::info!("Received ping, sending pong");
            if let Err(e) = write.send(Message::Pong(p)).await {
                log::error!("Failed to send pong: {e}");
            }
            (true, vec![])
        }

        Message::Pong(_) => {
            log::info!("Received pong");
            // Send any pong response messages (e.g., re-subscribe)
            for msg in exchange.on_pong_messages(symbols) {
                if let Err(e) = write.send(Message::Text(msg.into())).await {
                    log::error!("Failed to send pong response: {e}");
                }
            }
            (true, vec![])
        }

        Message::Close(frame) => {
            log::warn!("{} WS closed: {:?}", exchange.name(), frame);
            (false, vec![])
        }

        Message::Text(text) => {
            // Let the exchange parse the message (may return multiple updates)
            let ws_messages = exchange.parse_ws_message(&text);

            if ws_messages.is_empty() {
                return (true, vec![]);
            }

            let mut updates = Vec::new();
            let mut keep_alive = true;

            for ws_msg in ws_messages {
                match ws_msg {
                    WsMessage::Ping => {
                        // Send text-based pong
                        let pong = r#"{"type":"pong"}"#;
                        if let Err(e) = write.send(Message::Text(pong.into())).await {
                            log::error!("Failed to send text pong: {e}");
                        }
                    }

                    WsMessage::Pong => {
                        log::info!("Received text pong");
                        // Send any pong response messages (e.g., re-subscribe)
                        for msg in exchange.on_pong_messages(symbols) {
                            if let Err(e) = write.send(Message::Text(msg.into())).await {
                                log::error!("Failed to send pong response: {e}");
                            }
                        }
                    }

                    WsMessage::FundingUpdate {
                        symbol,
                        mark_price,
                        funding_rate,
                        timestamp_ms,
                    } => {
                        updates.push((symbol, mark_price, funding_rate, timestamp_ms));
                    }

                    WsMessage::Close => {
                        keep_alive = false;
                    }

                    WsMessage::Unknown(_) => {
                        // Ignore unknown messages
                    }
                }
            }

            (keep_alive, updates)
        }

        Message::Binary(_) => {
            // Binary messages not typically used for funding feeds
            (true, vec![])
        }

        Message::Frame(_) => (true, vec![]),
    }
}

// /// Update the live market feed state.
// async fn update_live_state(
//     live_market_feed: &Arc<RwLock<LiveMarketFeed>>,
//     perp_exchange: &PerpetualExchange,
//     snapshot: &HashMap<String, (f64, f64, i64)>,
// ) {
//     let mut state = live_market_feed.write().await;
//     for (symbol, (mark_px, funding, _)) in snapshot.iter() {
//         match perp_exchange {
//             PerpetualExchange::Hyperliquid => {
//                 state
//                     .hyperliquid
//                     .insert(symbol.clone(), (*mark_px, *funding));
//             }
//             PerpetualExchange::Pacifica => {
//                 state.pacifica.insert(symbol.clone(), (*mark_px, *funding));
//             }
//             PerpetualExchange::Lighter => {
//                 state.lighter.insert(symbol.clone(), (*mark_px, *funding));
//             }
//         }
//     }
// }

/// Send a live update to the FeedManager via bounded mpsc.
async fn send_live_update(
    feed_tx: &mpsc::Sender<MarketFeedUpdate>,
    perp_exchange: &PerpetualExchange,
    snapshot: &HashMap<String, (f64, f64, i64)>,
) {
    let data: HashMap<String, (f64, f64)> = snapshot
        .iter()
        .map(|(sym, (mark, fund, _))| (sym.clone(), (*mark, *fund)))
        .collect();

    let update = MarketFeedUpdate {
        exchange: perp_exchange.clone(),
        data,
    };

    if let Err(e) = feed_tx.send(update).await {
        log::error!("FeedManager receiver dropped, cannot send update: {e}");
    }
}

/// Write funding rates to the database.
async fn write_funding_to_database(
    db_conn: &Arc<PgPool>,
    perp_exchange: &PerpetualExchange,
    snapshot: &HashMap<String, (f64, f64, i64)>,
) {
    let funding_rates: Vec<FundingRate> = snapshot
        .iter()
        .map(|(symbol, (mark_px, funding, _))| FundingRate {
            id: Uuid::new_v4(),
            platform: perp_exchange.to_string(),
            symbol: symbol.clone(),
            mark_px: *mark_px,
            rate: *funding,
            timestamp: Utc::now().naive_utc(),
        })
        .collect();

    if funding_rates.is_empty() {
        return;
    }

    let db = db_conn.clone();
    tokio::spawn(async move {
        if let Err(e) = insert_funding_rates(db, funding_rates).await {
            log::warn!("DB insert failed: {e:?}");
        }
    });
}
