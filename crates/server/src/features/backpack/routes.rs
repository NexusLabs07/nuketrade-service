use axum::routing::post;

use crate::{AppState, features::backpack::controller::deposit};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/deposit", post(deposit))
}
