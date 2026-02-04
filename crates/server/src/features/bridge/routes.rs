use axum::routing::post;

use crate::{
    AppState,
    features::bridge::controller::{execute_permits, get_quote},
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/quote", post(get_quote))
        .route("/execute/permits", post(execute_permits))
}
