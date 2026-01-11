use axum::routing::post;

use crate::{
    AppState,
    controller::hyperliquid::{
        cancel_perp_order, close_all_perp_position, close_perp_position, create_perp_position,
    },
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/create-perp-position", post(create_perp_position))
        .route("/close-perp-position", post(close_perp_position))
        .route("/close-all-perp-position", post(close_all_perp_position))
        .route("/cancel-perp-position", post(cancel_perp_order))
}
