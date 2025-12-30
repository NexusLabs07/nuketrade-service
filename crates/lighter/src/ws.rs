use db::{crud::insert_funding_rate, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use crate::{LIGHTER_WS_URL, types::MarketStatsMsg};
use core::{
    funding::{Dex, FundingSnapshot},
    token_list::TOKEN_LIST,
};
use std::{collections::HashSet, sync::Arc};

pub async fn start_lighter_funding_feed(db_conn: Arc<PgPool>) {
    loop {
        let (ws_stream, _) = match connect_async(LIGHTER_WS_URL).await {
            Ok((ws_stream, resp)) => (ws_stream, resp),
            Err(e) => {
                log::error!("Error connecting to Lighter WS: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        log::info!("Connected to Lighter WS");

        let (mut write, mut read) = ws_stream.split();

        let sub = json!({
            "type": "subscribe",
            "channel": "market_stats/all"
        });

        match write.send(sub.to_string().into()).await {
            Ok(_) => log::info!("Subscribed to market stats"),
            Err(e) => log::error!("Error subscribing to market stats: {}", e),
        };

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

            let parsed: MarketStatsMsg = match serde_json::from_str(msg) {
                Ok(parsed) => parsed,
                Err(e) => {
                    log::error!("Error parsing message: {}", e);
                    continue;
                }
            };

            let market_id = parsed.market_stats.market_id.to_string();

            let funding_8h: f64 = match parsed.market_stats.funding_rate.parse() {
                Ok(val) => val,
                Err(_) => {
                    log::warn!("Failed to parse funding rate");
                    continue;
                }
            };

            // Lighter funding rate is 8h, so we need to convert to hourly
            let funding_hr: f64 = funding_8h / 8.0;

            let mark_px = match parsed.market_stats.mark_price.parse() {
                Ok(val) => val,
                Err(_) => {
                    log::warn!("Failed to parse market price for {}", market_id);
                    continue;
                }
            };

            let snapshot = FundingSnapshot {
                dex: Dex::Lighter,
                coin: String::from("TEST"),
                funding_hr,
                mark_price: mark_px,
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            };

            let funding_rate = FundingRate {
                id: Uuid::new_v4(),
                platform: Dex::Lighter.to_string(),
                symbol: String::from("TEST"), // TODO: Need to create a mapping for Lighter market_id -> coin
                rate: funding_hr,
                created_at: chrono::Utc::now(),
                timestamp: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            if let Err(err) = insert_funding_rate(db_conn.clone(), funding_rate).await {
                log::warn!(
                    "Failed to insert funding rate. Failed with error: {:?}",
                    err
                );
            };

            log::info!("snapshot {:?}", snapshot);
        }
    }
}
