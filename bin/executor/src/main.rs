use executor::{SevenDayApr, cron::calculate_best_pair};
use hyperliquid::helpers::markets::HL_MARKETS;
use pacifica::helpers::markets::PACIFICA_MARKETS;
use perp_core::{config::Config, exchange::PerpetualExchange, types::MarketFeedUpdate};
use std::{collections::HashMap, sync::Arc};
use tokio_cron_scheduler::JobScheduler;

use anyhow::Context;
use db::connect_db;
use server::{
    run_server,
    types::{FeedSnapshot, LiveMarketFeedResponse, MarketFeedValueStruct},
};
use tokio::{
    sync::{mpsc, watch},
    time::{Duration, MissedTickBehavior},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!("Error loading config from env. Failed with error: {err:?}");
            std::process::exit(1);
        }
    };

    //wrap these in a single function that runs the node
    log::info!("Engine starting....");

    log::info!("Running DB migrations....");
    tokio::task::spawn_blocking(|| {
        if let Err(err) = db::run_db_migrations() {
            log::error!("Error running DB migrations. Failed with error: {err:?}");
            std::process::exit(1);
        };
    })
    .await
    .expect("Blocking task panicked");

    log::info!("Connecting to DB....");
    let db = connect_db(config.db_url.as_str())
        .await
        .context("Failed to connect with DB")?;

    let (seven_day_apr_tx, seven_day_apr_rx) = watch::channel(SevenDayApr {
        seven_day_avg_apr: HashMap::new(),
        seven_day_spread_apr: HashMap::new(),
    });

    let scheduler = JobScheduler::new().await?;
    calculate_best_pair(db.clone(), scheduler, seven_day_apr_tx).await?;

    // let live_market_feed = Arc::new(RwLock::new(LiveMarketFeed {
    //     hyperliquid: HashMap::new(),
    //     lighter: HashMap::new(),
    //     pacifica: HashMap::new(),
    // }));

    let hl_leverage: HashMap<String, u32> = HL_MARKETS
        .iter()
        .map(|m| (m.name.clone(), m.max_leverage))
        .collect();

    let pacifica_leverage: HashMap<String, u32> = PACIFICA_MARKETS
        .iter()
        .map(|m| (m.symbol.to_string(), m.max_leverage))
        .collect();

    let (feed_tx, feed_rx) = mpsc::channel::<MarketFeedUpdate>(256);

    let initial_snapshot = Arc::new(FeedSnapshot {
        by_symbol: HashMap::new(),
        formatted: Vec::new(),
    });
    let (watch_tx, watch_rx) = watch::channel(initial_snapshot);

    tokio::spawn(run_feed_manager(
        feed_rx,
        watch_tx,
        hl_leverage,
        pacifica_leverage,
    ));

    log::info!("Starting Hyperliquid live feed....");
    let db_clone_1 = db.clone();
    let feed_tx_clone = feed_tx.clone();
    tokio::spawn(async move {
        hyperliquid::start_hl_funding_feed(db_clone_1, feed_tx_clone).await;
    });

    log::info!("Starting Pacifica live feed....");
    let db_clone_2 = db.clone();
    let feed_tx_clone_2 = feed_tx.clone();
    tokio::spawn(async move {
        pacifica::start_pacifica_funding_feed(db_clone_2, feed_tx_clone_2).await;
    });

    drop(feed_tx);

    run_server(config, db, watch_rx, seven_day_apr_rx).await?;

    Ok(())
}

async fn run_feed_manager(
    mut feed_rx: mpsc::Receiver<MarketFeedUpdate>,
    watch_tx: watch::Sender<Arc<FeedSnapshot>>,
    hl_leverage: HashMap<String, u32>,
    pacifica_leverage: HashMap<String, u32>,
) {
    let mut by_symbol: HashMap<String, LiveMarketFeedResponse> = HashMap::new();
    let mut dirty = false;

    let mut publish_tick = tokio::time::interval(Duration::from_millis(200));
    publish_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            maybe_update = feed_rx.recv() => {
                match maybe_update {
                    Some(update) => {
                        let mut changed = false;

                        for (symbol, (mark_px, funding_rate)) in update.data {
                            match &update.exchange {
                                PerpetualExchange::Hyperliquid => {
                                    let entry = by_symbol
                                        .entry(symbol.clone())
                                        .or_insert_with(|| LiveMarketFeedResponse {
                                            symbol: symbol.clone(),
                                            hyperliquid: None,
                                            pacifica: None,
                                        });

                                    entry.hyperliquid = Some(MarketFeedValueStruct {
                                        mark_px: Some(mark_px),
                                        funding: Some(funding_rate),
                                        max_leverage: hl_leverage.get(&symbol).copied(),
                                    });
                                    changed = true;
                                }
                                PerpetualExchange::Pacifica => {
                                    let entry = by_symbol
                                        .entry(symbol.clone())
                                        .or_insert_with(|| LiveMarketFeedResponse {
                                            symbol: symbol.clone(),
                                            hyperliquid: None,
                                            pacifica: None,
                                        });

                                    entry.pacifica = Some(MarketFeedValueStruct {
                                        mark_px: Some(mark_px),
                                        funding: Some(funding_rate),
                                        max_leverage: pacifica_leverage.get(&symbol).copied(),
                                    });
                                    changed = true;
                                }
                                PerpetualExchange::Lighter => {

                                }
                            }
                        }

                        if changed {
                            dirty = true;
                        }
                    }
                    None => {
                        log::info!("All feed senders dropped, FeedManager shutting down");
                        break;
                    }
                }
            }

            _ = publish_tick.tick() => {
                if !dirty {
                    continue;
                }
                dirty = false;

                let mut formatted: Vec<LiveMarketFeedResponse> = by_symbol.values().cloned().collect();
                formatted.sort_unstable_by(|a, b| a.symbol.cmp(&b.symbol));

                let snapshot = Arc::new(FeedSnapshot {
                    by_symbol: by_symbol.clone(),
                    formatted,
                });

                if watch_tx.send(snapshot).is_err() {
                    log::warn!("All feed receivers dropped, FeedManager shutting down");
                    break;
                }
            }
        }
    }
}

// /// Build the pre-formatted API response from raw state + precomputed leverage maps.
// /// Matches the exact contract: `Vec<{ symbol, hyperliquid: Option, pacifica: Option }>`.
// fn build_formatted_response(
//     raw: &RawMarketData,
//     hl_leverage: &HashMap<String, u32>,
//     pacifica_leverage: &HashMap<String, u32>,
// ) -> Vec<LiveMarketFeedResponse> {
//     let mut merged: HashMap<
//         String,
//         (Option<MarketFeedValueStruct>, Option<MarketFeedValueStruct>),
//     > = HashMap::new();

//     for (symbol, (mark_px, funding_rate)) in &raw.hyperliquid {
//         let entry = merged.entry(symbol.clone()).or_insert((None, None));
//         entry.0 = Some(MarketFeedValueStruct {
//             mark_px: Some(*mark_px),
//             funding: Some(*funding_rate),
//             max_leverage: hl_leverage.get(symbol).copied(),
//         });
//     }

//     for (symbol, (mark_px, funding_rate)) in &raw.pacifica {
//         let entry = merged.entry(symbol.clone()).or_insert((None, None));
//         entry.1 = Some(MarketFeedValueStruct {
//             mark_px: Some(*mark_px),
//             funding: Some(*funding_rate),
//             max_leverage: pacifica_leverage.get(symbol).copied(),
//         });
//     }

//     merged
//         .into_iter()
//         .map(|(symbol, (hl, pac))| LiveMarketFeedResponse {
//             symbol,
//             hyperliquid: hl,
//             pacifica: pac,
//         })
//         .collect()
// }
