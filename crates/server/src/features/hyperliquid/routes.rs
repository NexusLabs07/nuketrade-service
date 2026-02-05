use axum::routing::{get, post};

use crate::{
    AppState,
    features::hyperliquid::controller::{
        bridge_to_hyperliquid, get_perp_metadata, get_spot_metadata, get_user_open_positions,
    },
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/spot-metadata", get(get_spot_metadata))
        .route("/perp-metadata", get(get_perp_metadata))
        .route(
            "/open-positions/{user_evm_address}",
            get(get_user_open_positions),
        )
        .route("/deposit", post(bridge_to_hyperliquid))
}
