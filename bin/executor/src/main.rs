use anyhow::Context;
use db::connect_db;
use executor::config::Config;

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

    log::info!("Starting Hyperliquid funding feed....");

    log::info!("Running DB migrations....");

    // tokio::spawn(async move {
    hyperliquid::start_hl_funding_feed(db.clone()).await; // @Vaibhav - is db.clone() correct?
    lighter::start_lighter_funding_feed(db.clone()).await;
    // });
    Ok(())
}
