use anyhow::Context;
use db::connect_db;
use executor::config::Config;
use hyperliquid::helpers::markets::HL_MARKETS;
use pacifica::helpers::markets::PACIFICA_MARKETS;
use perp_core::{Dex, MarketFeedUpdate, RawMarketData};
use server::{
    FeedSnapshot,
    controller::aggregated::{LiveMarketFeedResponse, MarketFeedValueStruct},
    run_server,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            log::error!(
                "Error loading config from env. Failed with error: {:?}",
                err
            );
            std::process::exit(1);
        }
    };

    //wrap these in a single function that runs the node
    log::info!("Engine starting....");

    log::info!("Running DB migrations....");
    tokio::task::spawn_blocking(|| {
        if let Err(err) = db::run_db_migrations() {
            log::error!("Error running DB migrations. Failed with error: {:?}", err);
            std::process::exit(1);
        };
    })
    .await
    .expect("Blocking task panicked");

    log::info!("Connecting to DB....");
    let db = connect_db(config.db_url.as_str())
        .await
        .context("Failed to connect with DB")?;

    // --- Precompute leverage maps once at startup (0(1) lookup later) ---
    let hl_leverage: HashMap<String, u32> = HL_MARKETS
        .iter()
        .map(|m| (m.name.clone(), m.max_leverage))
        .collect();

    let pacifica_leverage: HashMap<String, u32> = PACIFICA_MARKETS
        .iter()
        .map(|m| (m.symbol.to_string(), m.max_leverage))
        .collect();

    // --- Channel setup ---
    // Bounded mpsc: WS tasks -> FeedManager. Backpressure via send().await.
    let (feed_tx, feed_rx) = mpsc::channel::<MarketFeedUpdate>(256);

    // Single watch channel: FeedManager -> all readers.
    // Both raw + formatted published atomically per tick -> no version skew.
    let initial_snapshot = Arc::new(FeedSnapshot {
        raw: RawMarketData::default(),
        formatted: Vec::new(),
    });

    let (watch_tx, watch_rx) = watch::channel(initial_snapshot);

    // --- Spawn FeedManager background task ---
    tokio::spawn(run_feed_manager(
        feed_rx,
        watch_tx,
        hl_leverage,
        pacifica_leverage,
    ));

    // log::info!("Starting Hyperliquid live feed....");
    // let db_clone_1 = db.clone();
    // let live_market_feed_clone = live_market_feed.clone();
    // tokio::spawn(async move {
    //     hyperliquid::start_hl_funding_feed(db_clone_1.clone(), live_market_feed_clone.clone())
    //         .await;
    // });

    // log::info!("Starting Pacifica live feed....");
    // let db_clone_2 = db.clone();
    // let live_market_feed_clone_2 = live_market_feed.clone();
    // tokio::spawn(async move {
    //     pacifica::start_pacifica_funding_feed(db_clone_2.clone(), live_market_feed_clone_2.clone())
    //         .await;
    // });

    // log::info!("Starting Lighter funding feed....");
    // let db_clone_2 = db.clone();
    // let platfroms_funding_rate_clone_2 = platforms_funding_rate.clone();
    // tokio::spawn(async move {
    //     lighter::start_lighter_funding_feed(
    //         db_clone_2.clone(),
    //         platfroms_funding_rate_clone_2.clone(),
    //     )
    //     .await;
    // });

    drop(feed_tx);
    run_server(db, watch_rx).await?;

    Ok(())
}

/// Central Feedmanager: drains mpsc, coalesces on 200ms tick, rebuilds
/// both raw state + formatted API response, publishes atomically via
/// a single watch channel
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
            update = feed_rx.recv() => {
                let Some(update) = update else {
                    log::info!("All feed senders dropped, FeedManager shutting down");
                    break;
                };

                match update.dex {
                    Dex::Hyperliquid => {
                        for (sym, val) in update.data {
                            raw.hyperliquid.insert(sym, val);
                        }
                    }
                    Dex::Pacifica => {
                        for (sym, val) in update.data {
                            raw.pacifica.insert(sym, val);
                        }
                    }
                    Dex::Lighter => {
                        for (sym, val) in update.data {
                            raw.lighter.insert(sym, val);
                        }
                    }
                }
                dirty = true;
            }

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
