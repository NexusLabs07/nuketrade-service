// use axum::{Json, extract::State};
// use perp_core::types::LiveMarketFeed;

// use crate::AppState;

pub mod aggregated;
pub mod bridge;
pub mod hyperliquid;
pub mod pacifica;
pub mod user;

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

// pub async fn get_live_market_feed(State(state): State<AppState>) -> Json<LiveMarketFeed> {
//     let snapshot = state.live_market_feed.read().await;
//     Json(snapshot.clone())
// }
