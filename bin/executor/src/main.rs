use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use backpack::BackpackExchange;
use tokio::sync::{mpsc, watch};
use tokio_cron_scheduler::JobScheduler;

use db::connect_db;
use executor::{SevenDayApr, cron::calculate_best_pair, feed_manager::run_feed_manager};
use hyperliquid::helpers::markets::HL_MARKETS;
use pacifica::helpers::markets::PACIFICA_MARKETS;
use perp_core::{MarketFeedUpdate, config::Config};
use server::{run_server, types::FeedSnapshot};

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

    let hl_leverage: HashMap<String, u32> = HL_MARKETS
        .iter()
        .map(|m| (m.name.clone(), m.max_leverage))
        .collect();

    let pacifica_leverage: HashMap<String, u32> = PACIFICA_MARKETS
        .iter()
        .map(|m| (m.symbol.to_string(), m.max_leverage))
        .collect();

    let backpack_leverage: HashMap<String, u32> =
        match BackpackExchange::new().fetch_max_leverage_map().await {
            Ok(map) => map,
            Err(err) => {
                log::warn!(
                    "Backpack: failed to fetch leverage metadata, continuing with empty map:{err}"
                );
                HashMap::new()
            }
        };

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
        backpack_leverage,
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

    log::info!("Starting Backpack live feed....");
    let db_clone_3 = db.clone();
    let feed_tx_clone_3 = feed_tx.clone();
    tokio::spawn(async move {
        backpack::start_backpack_funding_feed(db_clone_3, feed_tx_clone_3).await;
    });

    log::info!("Starting Lighter live feed....");
    let db_clone_4 = db.clone();
    let feed_tx_clone_4 = feed_tx.clone();
    tokio::spawn(async move {
        lighter::start_lighter_funding_feed(db_clone_4, feed_tx_clone_4).await;
    });

    drop(feed_tx);

    run_server(config, db, watch_rx, seven_day_apr_rx).await?;

    Ok(())
}
