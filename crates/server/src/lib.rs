use core::types::PlatformsFundingRate;
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::{
    controller::{
        add_to_waitlist, get_funding_rate, get_referral_count, get_total_points, get_total_users,
        get_user_position, root,
    },
    types::AppState,
};

pub mod controller;
pub mod error;
pub mod types;

pub async fn run_server(
    db: Arc<PgPool>,
    platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
) {
    let app_state = AppState {
        db,
        platforms_funding_rate,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/funding-rate", get(get_funding_rate))
        .route("/add-to-waitlist", post(add_to_waitlist))
        .route("/total-users", get(get_total_users))
        .route("/total-points/{user_id}", get(get_total_points))
        .route("/referral-count/{referral_code}", get(get_referral_count))
        .route("/user-position/{user_id}", get(get_user_position))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    log::info!("Starting Server...");

    axum::serve(listener, app).await.unwrap();
}
