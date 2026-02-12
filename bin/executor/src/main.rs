use executor::{SevenDayApr, cron::calculate_best_pair};
use hyperliquid::helpers::markets::HL_MARKETS;
use pacifica::helpers::markets::PACIFICA_MARKETS;
use perp_core::{
    RawMarketData, config::Config, exchange::PerpetualExchange, types::MarketFeedUpdate,
};
use std::{collections::HashMap, sync::Arc};
use tokio_cron_scheduler::JobScheduler;

use anyhow::Context;
use db::connect_db;
use server::{
    run_server,
    types::{FeedSnapshot, LiveMarketFeedResponse, MarketFeedValueStruct},
};
use tokio::sync::{mpsc, watch};

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
        raw: RawMarketData::default(),
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
    let mut raw = RawMarketData::default();
    let mut coalesce_tick = tokio::time::interval(std::time::Duration::from_millis(200));
    let mut dirty = false;

    loop {
        tokio::select! {
            // Drain incoming updates as they arrive
            maybe_update = feed_rx.recv() => {
                match maybe_update {
                    Some(update) => {
                        match update.exchange {
                            PerpetualExchange::Hyperliquid => {
                                for (sym, val) in update.data {
                                    raw.hyperliquid.insert(sym, val);
                                }
                            }
                            PerpetualExchange::Pacifica => {
                                for (sym, val) in update.data {
                                    raw.pacifica.insert(sym, val);
                                }
                            }
                            PerpetualExchange::Lighter => {
                                for (sym, val) in update.data {
                                    raw.lighter.insert(sym, val);
                                }
                            }
                        }
                        dirty = true;
                    }
                    // All senders dropped — channel closed
                    None => {
                        log::info!("All feed senders dropped, FeedManager shutting down");
                        break;
                    }
                }
            }

            // Coalescing tick: rebuild + publish only if something changed
            _ = coalesce_tick.tick() => {
                if !dirty {
                    continue;
                }
                dirty = false;

                let formatted = build_formatted_response(&raw, &hl_leverage, &pacifica_leverage);
                let snapshot = Arc::new(FeedSnapshot {
                    raw: raw.clone(),
                    formatted,
                });

                // watch::send only fails if all receivers dropped — server is gone
                if watch_tx.send(snapshot).is_err() {
                    log::warn!("All feed receivers dropped, shutting down FeedManager");
                    break;
                }
            }
        }
    }
}

/// Build the pre-formatted API response from raw state + precomputed leverage maps.
/// Matches the exact contract: `Vec<{ symbol, hyperliquid: Option, pacifica: Option }>`.
fn build_formatted_response(
    raw: &RawMarketData,
    hl_leverage: &HashMap<String, u32>,
    pacifica_leverage: &HashMap<String, u32>,
) -> Vec<LiveMarketFeedResponse> {
    let mut merged: HashMap<
        String,
        (Option<MarketFeedValueStruct>, Option<MarketFeedValueStruct>),
    > = HashMap::new();

    for (symbol, (mark_px, funding_rate)) in &raw.hyperliquid {
        let entry = merged.entry(symbol.clone()).or_insert((None, None));
        entry.0 = Some(MarketFeedValueStruct {
            mark_px: Some(*mark_px),
            funding: Some(*funding_rate),
            max_leverage: hl_leverage.get(symbol).copied(),
        });
    }

    for (symbol, (mark_px, funding_rate)) in &raw.pacifica {
        let entry = merged.entry(symbol.clone()).or_insert((None, None));
        entry.1 = Some(MarketFeedValueStruct {
            mark_px: Some(*mark_px),
            funding: Some(*funding_rate),
            max_leverage: pacifica_leverage.get(symbol).copied(),
        });
    }

    merged
        .into_iter()
        .map(|(symbol, (hl, pac))| LiveMarketFeedResponse {
            symbol,
            hyperliquid: hl,
            pacifica: pac,
        })
        .collect()
}
