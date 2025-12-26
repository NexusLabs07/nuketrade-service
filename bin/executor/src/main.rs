#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Funding arb engine starting....");
    hyperliquid::start_hl_funding_feed().await?;

    Ok(())
}
