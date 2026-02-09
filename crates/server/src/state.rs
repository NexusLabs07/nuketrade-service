use std::sync::Arc;

use perp_core::{LiveMarketFeed, SevenDayApr, config::Config};
use sqlx::PgPool;
use tokio::sync::{RwLock, watch};

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<PgPool>,
    pub live_market_feed: Arc<RwLock<LiveMarketFeed>>,
    pub seven_day_apr: watch::Receiver<SevenDayApr>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Arc<PgPool>,
        live_market_feed: Arc<RwLock<LiveMarketFeed>>,
        seven_day_apr: watch::Receiver<SevenDayApr>,
    ) -> Self {
        Self {
            config,
            db,
            live_market_feed,
            seven_day_apr,
        }
    }
}
