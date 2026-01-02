use db::{crud::insert_funding_rate, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use crate::{HYPERLIQUID_WS_URL, types::ActiveAssetCtxMsg};
use core::{
    funding::{Dex, FundingSnapshot},
    token_list::TOKEN_LIST,
};
use std::{sync::Arc, time::Instant};

pub async fn start_hl_funding_feed(db_conn: Arc<PgPool>) {
    let (ws_stream, _) = match connect_async(HYPERLIQUID_WS_URL).await {
        Ok((ws_stream, resp)) => (ws_stream, resp),
        Err(e) => {
            log::error!("Error connecting to Hyperliquid WS: {}", e);
            //Waits for 1 minute before retrying
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            //TODO: Add retry limit
            return;
        }
    };

    let mut timer = Instant::now();

    log::info!("Connected to Hyperliquid WS");

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

    while let Some(msg) = read.next().await {
        if let Err(err) = msg {
            log::error!("Error receiving message: {}", err);
            continue;
        };

        let msg = msg.unwrap();

        if !msg.is_text() {
            continue;
        }

        let msg = match msg.to_text() {
            Ok(text) => text,
            Err(e) => {
                log::error!("Error converting message to text: {}", e);
                continue;
            }
        };

        let parsed: ActiveAssetCtxMsg = match serde_json::from_str(msg) {
            Ok(parsed) => parsed,
            Err(e) => {
                log::error!("Error parsing message: {}", e);
                continue;
            }
        };

        let coin = parsed.data.coin.to_string();

        let funding_hr: f64 = match parsed.data.ctx.funding.parse() {
            Ok(val) => val,
            Err(_) => {
                log::warn!("Failed to parse funding rate for {}", coin);
                continue;
            }
        };

        let mark_px: f64 = match parsed.data.ctx.mark_px.parse() {
            Ok(val) => val,
            Err(_) => {
                log::warn!("Failed to parse mark price for {}", coin);
                continue;
            }
        };

        let snapshot = FundingSnapshot {
            dex: Dex::Hyperliquid,
            coin: parsed.data.coin.to_string(),
            funding_hr,
            mark_price: mark_px,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        let funding_rate = FundingRate {
            id: Uuid::new_v4(),
            platform: Dex::Hyperliquid.to_string(),
            symbol: snapshot.coin.clone(), //TODO: This needs to be normalised for different exchanges
            rate: funding_hr,
            mark_px: mark_px,
            timestamp: chrono::Utc::now(), //TODO: this is the timestamp received from ws
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        if timer.elapsed().as_secs() >= 60 * 30 {
            if let Err(err) = insert_funding_rate(db_conn.clone(), funding_rate).await {
                //TODO: Add retry logic and fail eventually
                log::warn!(
                    "Failed to insert funding rate. Failed with error: {:?}",
                    err
                );
            };

            //reset timer
            timer = Instant::now();
        }

        log::info!("snapshot {:?}", snapshot);
    }
}
