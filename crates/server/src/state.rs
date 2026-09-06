use std::sync::Arc;

use perp_core::{SevenDayApr, config::Config};
use sqlx::PgPool;
use tokio::sync::watch;

use crate::types::FeedSnapshot;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<PgPool>,
    pub feed: watch::Receiver<Arc<FeedSnapshot>>,
    pub seven_day_apr: watch::Receiver<SevenDayApr>,
    /// Network identity emitted with internal feed snapshots so the
    /// TypeScript API can fail closed when the two processes are misaligned.
    pub bulk_network: String,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Arc<PgPool>,
        feed: watch::Receiver<Arc<FeedSnapshot>>,
        seven_day_apr: watch::Receiver<SevenDayApr>,
        bulk_network: String,
    ) -> Self {
        Self {
            config,
            db,
            feed,
            seven_day_apr,
            bulk_network,
        }
    }
}
