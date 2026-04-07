use std::{collections::HashMap, sync::Arc, time::Duration};

use perp_core::{MarketFeedUpdate, exchange::PerpetualExchange};
use server::types::{FeedSnapshot, LiveMarketFeedResponse, MarketFeedValueStruct};
use tokio::{
    sync::{mpsc, watch},
    time::MissedTickBehavior,
};

pub async fn run_feed_manager(
    mut feed_rx: mpsc::Receiver<MarketFeedUpdate>,
    watch_tx: watch::Sender<Arc<FeedSnapshot>>,
    hl_leverage: HashMap<String, u32>,
    pacifica_leverage: HashMap<String, u32>,
    backpack_leverage: HashMap<String, u32>,
) {
    let mut by_symbol: HashMap<String, LiveMarketFeedResponse> = HashMap::new();
    let mut dirty = false;

    let mut publish_tick = tokio::time::interval(Duration::from_millis(200));
    publish_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            maybe_update = feed_rx.recv() => {
                match maybe_update {
                    Some(update) => {
                        let mut changed = false;

                        for (symbol, (mark_px, funding_rate)) in update.data {
                            let entry = by_symbol
                                .entry(symbol.clone())
                                .or_insert_with(|| LiveMarketFeedResponse {
                                    symbol: symbol.clone(),
                                    backpack: None,
                                    hyperliquid: None,
                                    pacifica: None,
                                });

                            let value = MarketFeedValueStruct {
                                mark_px: Some(mark_px),
                                funding: Some(funding_rate),
                                max_leverage: match &update.exchange {
                                    PerpetualExchange::Backpack => backpack_leverage.get(&symbol).copied(),
                                    PerpetualExchange::Hyperliquid => hl_leverage.get(&symbol).copied(),
                                    PerpetualExchange::Pacifica => pacifica_leverage.get(&symbol).copied(),
                                    PerpetualExchange::Lighter => None,
                                },
                            };

                            match &update.exchange {
                                PerpetualExchange::Backpack => entry.backpack = Some(value),
                                PerpetualExchange::Hyperliquid => entry.hyperliquid = Some(value),
                                PerpetualExchange::Pacifica => entry.pacifica = Some(value),
                                PerpetualExchange::Lighter => {}
                            }

                            changed = true;
                        }

                        if changed {
                            dirty = true;
                        }
                    }
                    None => {
                        log::info!("All feed senders dropped, FeedManager shutting down");
                        break;
                    }
                }
            }

            _ = publish_tick.tick() => {
                if !dirty {
                    continue;
                }
                dirty = false;

                let mut formatted: Vec<LiveMarketFeedResponse> = by_symbol.values().cloned().collect();
                formatted.sort_unstable_by(|a, b| a.symbol.cmp(&b.symbol));

                let snapshot = Arc::new(FeedSnapshot {
                    by_symbol: by_symbol.clone(),
                    formatted,
                });

                if watch_tx.send(snapshot).is_err() {
                    log::warn!("All feed receivers dropped, FeedManager shutting down");
                    break;
                }
            }
        }
    }
}
