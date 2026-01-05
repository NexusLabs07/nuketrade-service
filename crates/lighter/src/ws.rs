use db::{crud::insert_funding_rate, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    sync::RwLock,
    time::{Instant, interval},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::{LIGHTER_WS_URL, helpers::markets, types::MarketStatsMsg};
use core::{funding::Dex, token_list::TOKEN_LIST, types::PlatformsFundingRate};
use std::{collections::HashMap, sync::Arc, time::Duration};

pub async fn start_lighter_funding_feed(
    db_conn: Arc<PgPool>,
    platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
) {
    let (ws_stream, _) = match connect_async(LIGHTER_WS_URL).await {
        Ok((ws_stream, resp)) => (ws_stream, resp),
        Err(e) => {
            log::error!("Error connecting to Lighter WS: {}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            // TODO: Add retry limit
            return;
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
                                //TODO: Failed crash program here
                                break;
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

                for (symbol, (funding, mark_px)) in last_snapshot.iter() {
                    let funding_rate = FundingRate {
                        id: Uuid::new_v4(),
                        platform: Dex::Lighter.to_string(),
                        symbol: symbol.clone(),
                        rate: *funding,
                        mark_px: *mark_px,
                        timestamp: chrono::Utc::now(),
                    };

                let db = db_conn.clone();

                //TODO: insert in one db call, now for every token one call is made
                tokio::spawn(async move {
                    //TODO: insert into DB from platforms_funding_rate and not last_snapshot
                    if let Err(e) = insert_funding_rate(db, funding_rate).await {
                        log::warn!("DB insert failed: {:?}", e);
                    }
                });
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
) -> (bool, Option<(String, f64, f64)>) {
    match msg {
        //TODO: Handle constant pongs
        Message::Ping(p) => {
            write.send(Message::Pong(p)).await.ok();
            (true, None)
        }

        Message::Close(frame) => {
            log::warn!("Lighter WS closed: {:?}", frame);
            (false, None)
        }

        Message::Text(text) => {
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
