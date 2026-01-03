use core::types::PlatformsFundingRate;

use axum::{Json, extract::State};

use crate::types::AppState;

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub async fn get_funding_rate(State(state): State<AppState>) -> Json<PlatformsFundingRate> {
    let snapshot = state.platforms_funding_rate.read().await;
    Json(snapshot.clone())
}
