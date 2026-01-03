use core::types::PlatformsFundingRate;
use std::sync::Arc;

use arc_swap::ArcSwap;
use axum::{Router, routing::get};

use crate::{
    controller::{get_funding_rate, root},
    types::AppState,
};

pub mod controller;
pub mod types;

pub async fn run_server(platforms_funding_rate: Arc<ArcSwap<PlatformsFundingRate>>) {
    let app_state = AppState {
        platforms_funding_rate,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/funding-rate", get(get_funding_rate))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
