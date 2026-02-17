use std::sync::Arc;

use perp_core::{SevenDayApr, config::Config};
use sqlx::PgPool;
use tokio::sync::watch;

use crate::{services::auth::AuthService, types::FeedSnapshot};

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<PgPool>,
    pub feed: watch::Receiver<Arc<FeedSnapshot>>,
    pub seven_day_apr: watch::Receiver<SevenDayApr>,
    pub auth: AuthService,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Arc<PgPool>,
        feed: watch::Receiver<Arc<FeedSnapshot>>,
        seven_day_apr: watch::Receiver<SevenDayApr>,
        auth: AuthService,
    ) -> Self {
        Self {
            config,
            db,
            feed,
            seven_day_apr,
            auth,
        }
    }
}
