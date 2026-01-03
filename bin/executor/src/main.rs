use core::types::PlatformsFundingRate;
use std::sync::Arc;

use anyhow::Context;
use arc_swap::ArcSwap;
use db::connect_db;
use executor::config::Config;
use server::run_server;

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
    });

    log::info!("Connecting to DB....");
    let db = connect_db(config.db_url.as_str())
        .await
        .context("Failed to connect with DB")?;

    let platforms_funding_rate = Arc::new(ArcSwap::from_pointee(PlatformsFundingRate {
        hyperliquid: None,
        lighter: None,
    }));

    log::info!("Starting Hyperliquid funding feed....");
    let db_clone_1 = db.clone();
    let platforms_funding_rate_clone = platforms_funding_rate.clone();
    tokio::spawn(async move {
        hyperliquid::start_hl_funding_feed(
            db_clone_1.clone(),
            platforms_funding_rate_clone.clone(),
        )
        .await;
    });

    log::info!("Starting Lighter funding feed....");
    let platfroms_funding_rate_clone_2 = platforms_funding_rate.clone();
    tokio::spawn(async move {
        lighter::start_lighter_funding_feed(db.clone(), platfroms_funding_rate_clone_2.clone())
            .await;
    });

    run_server(platforms_funding_rate).await;

    Ok(())
}
