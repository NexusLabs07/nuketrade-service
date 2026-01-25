use db::{crud::insert_funding_rates, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::RwLock,
    time::{Instant, interval},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{HYPERLIQUID_WS_URL, types::ActiveAssetCtxMsg};
use core::{funding::Dex, token_list::TOKEN_LIST, types::PlatformsFundingRate};
use std::{collections::HashMap, sync::Arc, time::Duration};

pub async fn start_hl_funding_feed(
    db_conn: Arc<PgPool>,
    platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
) {
    const MAX_RETRIES: u32 = 3;
    let mut retry_count = 0;

    let ws_stream = loop {
        match connect_async(HYPERLIQUID_WS_URL).await {
            Ok((ws_stream, _)) => break ws_stream,
            Err(e) => {
                retry_count += 1;
                log::error!(
                    "Error connecting to Hyperliquid WS (attempt {}/{}): {}",
                    retry_count,
                    MAX_RETRIES,
                    e
                );

                if retry_count >= MAX_RETRIES {
                    log::error!("Max retries reached. Exiting Hyperliquid feed.");
                    return;
                }

                log::info!("Retrying in 60 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }
    };

    log::info!("Connected to Hyperliquid WS");

    let mut db_tick = interval(Duration::from_secs(30 * 60));
    let mut state_tick = interval(Duration::from_secs(5));

    let mut last_snapshot: HashMap<String, (f64, f64)> = HashMap::new();
    let mut last_update = Instant::now();

    let (mut write, mut read) = ws_stream.split();

    for i in 0..TOKEN_LIST.len() {
        let sub = json!({
            "method": "subscribe",
            "subscription": {
                "type": "activeAssetCtx",
                "coin": TOKEN_LIST[i]
            }
        });

        match write.send(sub.to_string().into()).await {
            Ok(_) => log::info!("Subscribed to {}", TOKEN_LIST[i]),
            Err(e) => log::error!("Error subscribing to {}: {}", TOKEN_LIST[i], e),
        };
    }

    loop {
        tokio::select! {
            Some(msg_res) = read.next() => {
                match msg_res {
                    Ok(msg) => {
                        let (keep_alive, new_snapshot) = handle_ws_message(msg, &mut write).await;
                        if let Some((symbol, funding, mark_px)) = new_snapshot {

                            log::info!("Hyperliquid new snapshot for token: {:?}", symbol.clone());
                            last_update = Instant::now();
                            last_snapshot.insert(symbol, (funding, mark_px));
                        }
                        if !keep_alive {
                            log::warn!("Connection failed with Hyperliquid WS. Crashing program...");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        log::error!("Hyperliquid WS read error: {}", e);
                    }
                }
            },
            _ = state_tick.tick() => {
                let mut state = platforms_funding_rate.write().await;
                for(symbol, (funding, _)) in last_snapshot.iter() {
                    state.hyperliquid.insert(symbol.clone(), *funding);
                }
            },
            _ = db_tick.tick() => {
                    if last_update.elapsed() > Duration::from_secs(60) {
                        log::warn!("Skipping DB write: Hyperliquid data is stale");
                        continue;
                    }

                let mut funding_rate_vec = Vec::new();


                for (symbol, (funding, mark_px)) in last_snapshot.iter() {
                    let funding_rate = FundingRate {
                        id: Uuid::new_v4(),
                        platform: Dex::Hyperliquid.to_string(),
                        symbol: symbol.clone(),
                        rate: *funding,
                        mark_px: *mark_px,
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
    match msg {
        Message::Ping(p) => {
            write.send(Message::Pong(p)).await.ok();
            (true, None)
        }

        Message::Close(frame) => {
            log::warn!("Hyperliquid WS closed: {:?}", frame);
            (false, None)
        }

        Message::Text(text) => {
            let parsed: ActiveAssetCtxMsg = match serde_json::from_str(text.as_str()) {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            let coin = parsed.data.coin.to_string();

            let funding_hr: f64 = match parsed.data.ctx.funding.parse() {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            let mark_px: f64 = match parsed.data.ctx.mark_px.parse() {
                Ok(v) => v,
                Err(_) => return (true, None),
            };

            (true, Some((coin, funding_hr, mark_px)))
        }

        _ => (true, None),
    }
}
