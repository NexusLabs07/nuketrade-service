use axum::{Json, extract::State};
use core::types::PlatformsFundingRate;

use crate::AppState;

pub mod hyperliquid;
pub mod user;

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub async fn get_funding_rate(State(state): State<AppState>) -> Json<PlatformsFundingRate> {
    let snapshot = state.platforms_funding_rate.read().await;
    Json(snapshot.clone())
}
