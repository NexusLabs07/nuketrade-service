use std::sync::Arc;

use perp_core::{LiveMarketFeed, SevenDayApr, config::Config};
use sqlx::PgPool;
use tokio::sync::{RwLock, watch};

use std::net::SocketAddr;

use crate::{app::create_app, state::AppState};

pub mod app;
pub mod error;
pub mod features;
pub mod middleware;
pub mod services;
pub mod state;
pub mod types;

pub async fn run_server(
    config: Config,
    db: Arc<PgPool>,
    live_market_feed: Arc<RwLock<LiveMarketFeed>>,
    seven_day_apr: watch::Receiver<SevenDayApr>,
) -> anyhow::Result<()> {
    let app_state = AppState::new(config, db, live_market_feed, seven_day_apr);

    let app = create_app(app_state);

    let bind_addr =
        std::env::var("SERVER_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    log::info!("Starting Server on {bind_addr}...");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
