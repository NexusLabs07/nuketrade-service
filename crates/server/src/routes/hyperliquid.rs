use axum::routing::get;

use crate::{
    AppState,
    controller::hyperliquid::{get_perp_metadata, get_spot_metadata},
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/spot-metadata", get(get_spot_metadata))
        .route("/perp-metadata", get(get_perp_metadata))
}
