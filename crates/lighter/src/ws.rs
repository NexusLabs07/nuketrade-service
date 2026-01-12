use db::{crud::insert_funding_rates, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::RwLock,
    time::{Instant, interval},
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};
use uuid::Uuid;

use crate::{LIGHTER_WS_URL, helpers::markets, types::MarketStatsMsg};
use core::{funding::Dex, token_list::TOKEN_LIST, types::PlatformsFundingRate};
use std::{collections::HashMap, sync::Arc, time::Duration};

pub async fn start_lighter_funding_feed(
    db_conn: Arc<PgPool>,
    platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
) {
    const MAX_RETRIES: u32 = 3;
    let mut retry_count = 0;

    let ws_config = WebSocketConfig::default();

    let ws_stream = loop {
        match connect_async_with_config(LIGHTER_WS_URL, Some(ws_config.clone()), true).await {
            Ok((ws_stream, _)) => break ws_stream,
            Err(e) => {
                retry_count += 1;
                log::error!(
                    "Error connecting to Lighter WS (attempt {}/{}): {}",
                    retry_count,
                    MAX_RETRIES,
                    e
                );

                if retry_count >= MAX_RETRIES {
                    log::error!("Max retries reached. Exiting Lighter feed.");
                    return;
                }

                log::info!("Retrying in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    };

    log::info!("Connected to Lighter WS");

    let mut db_tick = interval(Duration::from_secs(30 * 60));
    let mut state_tick = interval(Duration::from_secs(5));

    let mut last_snapshot: HashMap<String, (f64, f64)> = HashMap::new();
    let mut last_update = Instant::now();

    let (mut write, mut read) = ws_stream.split();

    for i in 0..TOKEN_LIST.len() {
        let lighter_market = markets::MARKETS
            .iter()
            .find(|&x| x.symbol.eq(TOKEN_LIST[i]));

        if lighter_market.is_none() {
            continue;
        }

        let sub = json!({
            "type": "subscribe",
            "channel": format!("{}{}", "market_stats/", lighter_market.unwrap().market_index)
        });

        match write.send(sub.to_string().into()).await {
            Ok(_) => log::info!("Subscribed to market stats"),
            Err(e) => log::error!("Error subscribing to market stats: {}", e),
        };
    }

    loop {
        tokio::select! {
                Some(msg_res) = read.next() => {
                    match msg_res {
                        Ok(msg) => {
                            let (keep_alive, new_snapshot) = handle_ws_message(msg, &mut write).await;
                            if let Some((symbol, funding, mark_px)) = new_snapshot {
                                log::info!("Lighter new snapshot for token: {:?}", symbol.clone());
                                last_update = Instant::now();
                                last_snapshot.insert(symbol, (funding, mark_px));
                            }
                            if !keep_alive {
                                log::warn!("Connection failed with Lighter WS. Crashing program...");
                                std::process::exit(1);
                            }
                        },
                        Err(e) => {
                            log::error!("Lighter WS read error: {}", e);
                        }
                    }
                },
                _ = state_tick.tick() => {
                    let mut state = platforms_funding_rate.write().await;
                    for (symbol, (funding, _)) in last_snapshot.iter() {
                        state.lighter.insert(symbol.clone(), *funding);
                    }
                },
                _ = db_tick.tick() => {
                    if last_update.elapsed() > Duration::from_secs(60) {
                        log::warn!("Skipping DB write: Lighter data is stale");
                        continue;
                    }

                let mut funding_rate_vec = Vec::new();

                for (symbol, (funding, mark_px)) in last_snapshot.iter() {
                    let funding_rate = FundingRate {
                        id: Uuid::new_v4(),
                        platform: Dex::Lighter.to_string(),
                        symbol: symbol.clone(),
                        rate: *funding,
                        mark_px: *mark_px,
                        timestamp: chrono::Utc::now().naive_utc(),
                    };

                funding_rate_vec.push(funding_rate);
            }

            let db = db_conn.clone();

            tokio::spawn(async move {
                    if let Err(e) = insert_funding_rates(db, funding_rate_vec).await {
                        log::warn!("DB insert failed: {:?}", e);
                    }
            });

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
) -> (bool, Option<(String, f64, f64)>) {
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
            log::warn!("Lighter WS closed: {:?}", frame);
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
            let parsed: MarketStatsMsg = match serde_json::from_str(text.as_str()) {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            let market = match markets::MARKETS
                .iter()
                .find(|m| m.market_index == parsed.market_stats.market_id)
            {
                Some(m) => m,
                None => return (true, None),
            };

            let funding_8h: f64 = match parsed.market_stats.funding_rate.parse() {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            let mark_px: f64 = match parsed.market_stats.mark_price.parse() {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            (
                true,
                Some((market.symbol.to_string(), funding_8h / 8.0, mark_px)),
            )
        }

        _ => (true, None),
    }
}
