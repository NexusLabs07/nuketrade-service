use std::sync::Arc;

use perp_core::{LiveMarketFeed, config::Config};
use sqlx::PgPool;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<PgPool>,
    pub live_market_feed: Arc<RwLock<LiveMarketFeed>>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Arc<PgPool>,
        live_market_feed: Arc<RwLock<LiveMarketFeed>>,
    ) -> Self {
        Self {
            config,
            db,
            live_market_feed,
        }
    }
}
