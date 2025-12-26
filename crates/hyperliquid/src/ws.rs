use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

use crate::{HYPERLIQUID_WS_URL, types::ActiveAssetCtxMsg};
use core::{funding::{Dex, FundingSnapshot}, token_list::TOKEN_LIST};

pub async fn start_hl_funding_feed() -> anyhow::Result<()> {

    let (ws_stream, _) = connect_async(HYPERLIQUID_WS_URL).await?;
    log::info!("Connected to Hyperliquid WS");

    let (mut write, mut read) = ws_stream.split();

    for i in 0..TOKEN_LIST.len() {
        let sub = json!({
            "method": "subscribe",
            "subscription": {
                "type": "activeAssetCtx",
                "coin": TOKEN_LIST[i]
            }
        });
        
        write.send(sub.to_string().into()).await?;
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;

        if !msg.is_text() {
            continue;
        }

        let parsed: ActiveAssetCtxMsg = match serde_json::from_str(msg.to_text()?) {
            Ok(parsed) => parsed,
            Err(e) => {
                log::error!("Error parsing message: {}", e);
                continue;
            }
        };

        let funding_hr: f64 = parsed.data.ctx.funding.parse()?;
        let mark_px: f64 = parsed.data.ctx.mark_px.parse()?;

        let snapshot = FundingSnapshot {
            dex: Dex::Hyperliquid,
            coin: parsed.data.coin.to_string(),
            funding_hr,
            mark_price: mark_px,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        log::info!("snapshot {:?}", snapshot);
    }

    Ok(())
}
