use core::types::LiveMarketFeed;
use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use db::connect_db;
use executor::config::Config;
use server::run_server;
use tokio::sync::RwLock;

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

    let live_market_feed = Arc::new(RwLock::new(LiveMarketFeed {
        hyperliquid: HashMap::new(),
        lighter: HashMap::new(),
        pacifica: HashMap::new(),
    }));

    log::info!("Starting Hyperliquid live feed....");
    let db_clone_1 = db.clone();
    let live_market_feed_clone = live_market_feed.clone();
    tokio::spawn(async move {
        hyperliquid::start_hl_funding_feed(db_clone_1.clone(), live_market_feed_clone.clone())
            .await;
    });

    log::info!("Starting Pacifica live feed....");
    let db_clone_2 = db.clone();
    let live_market_feed_clone_2 = live_market_feed.clone();
    tokio::spawn(async move {
        pacifica::start_pacifica_funding_feed(db_clone_2.clone(), live_market_feed_clone_2.clone())
            .await;
    });

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

    run_server(db, live_market_feed).await;

    Ok(())
}
