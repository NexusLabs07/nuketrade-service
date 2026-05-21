use axum::routing::{get, post};

use crate::features::phoenix::controller::{
    build_market_order_transaction, get_perp_metadata, get_user_open_positions,
};

pub fn routes() -> axum::Router<crate::AppState> {
    axum::Router::new()
        .route(
            "/user/{user_address}/open-positions",
            get(get_user_open_positions),
        )
        .route("/perp-metadata", get(get_perp_metadata))
        .route(
            "/transaction/market-order",
            post(build_market_order_transaction),
        )
}
