use arc_swap::ArcSwap;
use db::{crud::insert_funding_rate, types::FundingRate};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::PgPool;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use crate::{LIGHTER_WS_URL, helpers::markets, types::MarketStatsMsg};
use core::{
    funding::{Dex, FundingSnapshot},
    token_list::TOKEN_LIST,
    types::PlatformsFundingRate,
};
use std::{sync::Arc, time::Instant};

pub async fn start_lighter_funding_feed(
    db_conn: Arc<PgPool>,
    platforms_funding_rate: Arc<ArcSwap<PlatformsFundingRate>>,
) {
    let (ws_stream, _) = match connect_async(LIGHTER_WS_URL).await {
        Ok((ws_stream, resp)) => (ws_stream, resp),
        Err(e) => {
            log::error!("Error connecting to Lighter WS: {}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            return;
        }
    };

    let mut timer = Instant::now();

    log::info!("Connected to Lighter WS");

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
            "channel": format!("{}{}", "market_stats/", lighter_market.unwrap().symbol)
        });

        match write.send(sub.to_string().into()).await {
            Ok(_) => log::info!("Subscribed to market stats"),
            Err(e) => log::error!("Error subscribing to market stats: {}", e),
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

        let parsed: MarketStatsMsg = match serde_json::from_str(msg) {
            Ok(parsed) => parsed,
            Err(e) => {
                log::error!("Error parsing message: {}", e);
                continue;
            }
        };

        let market_id = parsed.market_stats.market_id;

        let funding_8h: f64 = match parsed.market_stats.funding_rate.parse() {
            Ok(val) => val,
            Err(_) => {
                log::warn!("Failed to parse funding rate");
                continue;
            }
        };

        // Lighter funding rate is 8h, so we need to convert to hourly
        let funding_hr: f64 = funding_8h / 8.0;

        let market_info = markets::MARKETS
            .iter()
            .find(|&x| x.market_index == market_id);

        if market_info.is_none() {
            log::warn!("No token symbol found for market id: {}", market_id);
            continue;
        }

        let token_symbol = market_info.unwrap().symbol.to_string();

        let mark_px = match parsed.market_stats.mark_price.parse() {
            Ok(val) => val,
            Err(_) => {
                log::warn!("Failed to parse market price for {}", market_id);
                continue;
            }
        };

        let snapshot = FundingSnapshot {
            dex: Dex::Lighter,
            coin: token_symbol.clone(),
            funding_hr,
            mark_price: mark_px,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        let funding_rate = FundingRate {
            id: Uuid::new_v4(),
            platform: Dex::Lighter.to_string(),
            symbol: token_symbol.clone(),
            rate: funding_hr,
            mark_px: mark_px,
            created_at: chrono::Utc::now(),
            timestamp: chrono::Utc::now(), //TODO: timestamp from ws
            updated_at: chrono::Utc::now(),
        };

        //write the data into the state every 5-6 seconds
        if timer.elapsed().as_secs() % 5 == 0 || timer.elapsed().as_secs() % 5 == 1 {
            let pl_fr = (*platforms_funding_rate.load()).clone();

            let new_pl_fr = PlatformsFundingRate {
                hyperliquid: pl_fr.hyperliquid,
                lighter: Some(funding_hr),
            };

            platforms_funding_rate.store(Arc::new(new_pl_fr));
        }

        if timer.elapsed().as_secs() >= 60 * 30 {
            if let Err(err) = insert_funding_rate(db_conn.clone(), funding_rate).await {
                log::warn!(
                    "Failed to insert funding rate. Failed with error: {:?}",
                    err
                );
            };

            timer = Instant::now();
        }

        log::info!("snapshot {:?}", snapshot);
    }
}
