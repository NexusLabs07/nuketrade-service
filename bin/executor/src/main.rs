use db::connect_db;
use executor::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::get_config();

    //wrap these in a single function that runs the node
    log::info!("Engine starting....");

    log::info!("Running DB migrations....");
    db::run_db_migrations()?;

    log::info!("Connecting to DB....");
    let db = connect_db(config.db_url.as_str()).await?;

    log::info!("Starting Hyperliquid funding feed....");
    hyperliquid::start_hl_funding_feed().await?;

    Ok(())
}
