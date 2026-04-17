use axum::routing::get;

use crate::{AppState, features::lighter::controller::get_perp_metadata};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/perp-metadata", get(get_perp_metadata))
}
