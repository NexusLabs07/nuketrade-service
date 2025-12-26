#[tokio::main]
async fn main() -> anyhow::Result<()> {

    tracing_subscriber::fmt::init();

    log::info!("Engine starting....");

    hyperliquid::start_hl_funding_feed().await?;

    Ok(())
}
