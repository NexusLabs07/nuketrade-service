use db::{crud::insert_funding_rates, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::RwLock,
    time::{Instant, interval, interval_at},
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};
use uuid::Uuid;

use crate::PACIFICA_WS_URL;
use core::{funding::Dex, token_list::TOKEN_LIST, types::LiveMarketFeed};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct PricesMessage {
    pub channel: String,
    pub data: Vec<PriceData>,
}

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

pub async fn start_pacifica_funding_feed(
    db_conn: Arc<PgPool>,
    live_market_feed: Arc<RwLock<LiveMarketFeed>>,
) {
    const MAX_RETRIES: u32 = 3;
    let mut retry_count = 0;

    let ws_config = WebSocketConfig::default();

    let ws_stream = loop {
        match connect_async_with_config(PACIFICA_WS_URL, Some(ws_config.clone()), true).await {
            Ok((ws_stream, _)) => break ws_stream,
            Err(e) => {
                retry_count += 1;
                log::error!(
                    "Error connecting to Pacifica WS (attempt {}/{}): {}",
                    retry_count,
                    MAX_RETRIES,
                    e
                );

                if retry_count >= MAX_RETRIES {
                    log::error!("Max retries reached. Exiting Pacifica feed.");
                    return;
                }

                log::info!("Retrying in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    };

    log::info!("Connected to Pacifica WS");

    let mut db_tick: tokio::time::Interval = interval(Duration::from_secs(30 * 60));
    let mut state_tick = interval(Duration::from_secs(5));
    let mut ping_tick = interval_at(
        Instant::now() + Duration::from_secs(30),
        Duration::from_secs(30),
    );

    let mut last_snapshot: HashMap<String, (f64, f64)> = HashMap::new();
    let mut last_update = Instant::now();

    let (mut write, mut read) = ws_stream.split();

    let sub = json!({
        "method": "subscribe",
        "params": {
            "source": "prices"
        }
    });

    match write.send(sub.to_string().into()).await {
        Ok(_) => log::info!("Pacifica: Subscribed to market states"),
        Err(e) => log::error!("Error subscribing to market stats: {}", e),
    };

    loop {
        tokio::select! {
                Some(msg_res) = read.next() => {
                    match msg_res {
                        Ok(msg) => {
                            let (keep_alive, new_snapshot) = handle_ws_message(msg, &mut write).await;
                            if let Some(snapshot) = new_snapshot {
                                log::info!("Received new snapshot from Pacifica: {:?}", snapshot);
                                last_update = Instant::now();
                                for item in snapshot.into_iter() {
                                    last_snapshot.insert(item.0, (item.1, item.2));
                                }
                            }
                            if !keep_alive {
                                log::warn!("Connection failed with Pacifica WS. Crashing program...");
                                std::process::exit(1);
                            }
                        },
                        Err(e) => {
                            log::error!("Pacifica WS read error: {}", e);
                        }
                    }
                },
                _ = state_tick.tick() => {
                    let mut state = live_market_feed.write().await;
                    for (symbol, (mark_px, funding)) in last_snapshot.iter() {
                        state.pacifica.insert(symbol.clone(), (*mark_px, *funding));
                    }
                },
                _ = db_tick.tick() => {
                    if last_update.elapsed() > Duration::from_secs(60) {
                        log::warn!("Skipping DB write: Pacifica data is stale");
                        continue;
                    }

                let mut funding_rate_vec = Vec::new();

                for (symbol, (mark_px, funding)) in last_snapshot.iter() {
                    let funding_rate = FundingRate {
                        id: Uuid::new_v4(),
                        platform: Dex::Pacifica.to_string(),
                        symbol: symbol.clone(),
                        mark_px: *mark_px,
                        rate: *funding,
                    };

                funding_rate_vec.push(funding_rate);
            }

            let db = db_conn.clone();

            tokio::spawn(async move {
                    if let Err(e) = insert_funding_rates(db, funding_rate_vec).await {
                        log::warn!("DB insert failed: {:?}", e);
                    }
            });

            },
            _ = ping_tick.tick() => {
                let ping_msg = json!({"method": "ping"});
                if let Err(e) = write.send(Message::Text(ping_msg.to_string().into())).await {
                    log::error!("Failed to send heartbeat ping: {}", e);
                } else {
                    log::info!("Sent heartbeat ping to Pacifica");
                }
            }
        };
    }
}

async fn handle_ws_message(
    msg: Message,
    write: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
) -> (bool, Option<Vec<(String, f64, f64)>>) {
    log::debug!("Received WS message: {:?}", msg);

    match msg {
        Message::Ping(p) => {
            log::info!("Received ping from server, sending pong");
            if let Err(e) = write.send(Message::Pong(p)).await {
                log::error!("Failed to send pong: {}", e);
            }
            (true, None)
        }

        Message::Pong(_) => {
            log::info!("Received pong from server");
            (true, None)
        }

        Message::Close(frame) => {
            log::warn!("Pacifica WS closed: {:?}", frame);
            (false, None)
        }

        Message::Text(text) => {
            // Check if it's a text-based ping
            if text.contains("ping") {
                log::info!("Received text ping, sending text pong.");
                if let Err(e) = write.send(Message::Text(r#"{"type":"pong"}"#.into())).await {
                    log::error!("Failed to send text pong: {}", e);
                }
                return (true, None);
            }

            // Handle heartbeat pong response - re-subscribe to get fresh snapshot
            if text.contains(r#""channel":"pong""#) || text.contains(r#""channel": "pong""#) {
                log::info!("Received heartbeat pong from Pacifica, re-subscribing...");
                let resub = json!({
                    "method": "subscribe",
                    "params": {
                        "source": "prices"
                    }
                });
                if let Err(e) = write.send(Message::Text(resub.to_string().into())).await {
                    log::error!("Failed to re-subscribe after pong: {}", e);
                }
                return (true, None);
            }

            let parsed: PricesMessage = match serde_json::from_str(text.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Failed to parse Pacifica message: {} - raw: {}", e, text);
                    return (true, None);
                }
            };

            let tracked_tokens: Vec<(String, f64, f64)> = parsed
                .data
                .into_iter()
                .filter(|x| TOKEN_LIST.iter().any(|t| &x.symbol == t))
                .map(|m| {
                    (
                        m.symbol.to_string(),
                        m.mark.parse::<f64>().unwrap(),
                        m.funding.parse::<f64>().unwrap() / 8.0,
                    )
                })
                .collect();

            (true, Some(tracked_tokens))
        }

        _ => (true, None),
    }
}
