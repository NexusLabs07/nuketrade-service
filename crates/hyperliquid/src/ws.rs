use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

use crate::types::ActiveAssetCtxMsg;
use core::funding::{Dex, FundingSnapshot};

pub async fn start_hl_funding_feed() -> anyhow::Result<()> {
    let url = "wss://api.hyperliquid.xyz/ws";
    let (ws_stream, _) = connect_async(url).await?;
    println!("Connected to Hyperliquid WS");

    let (mut write, mut read) = ws_stream.split();

    let sub = json!({
        "method": "subscribe",
        "subscription": {
            "type": "activeAssetCtx",
            "coin": "ETH"
        }
    });

    write.send(sub.to_string().into()).await?;

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if !msg.is_text() {
            continue;
        }

        let parsed: ActiveAssetCtxMsg = match serde_json::from_str(msg.to_text()?) {
            Ok(parsed) => parsed,
            Err(e) => {
                println!("Error parsing message: {}", e);
                continue;
            }
        };

        let funding_hr: f64 = parsed.data.ctx.funding.parse()?;
        let mark_px: f64 = parsed.data.ctx.mark_px.parse()?;

        // APY = funding hr * 24 * 365 * 100
        let apy = funding_hr * 24.0 * 365.0 * 100.0;

        let snapshot = FundingSnapshot {
            dex: Dex::Hyperliquid,
            coin: parsed.data.coin.to_string(),
            funding_hr,
            apy,
            mark_price: mark_px,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        println!("{:?}", snapshot);
    }

    Ok(())
}
