use axum::{Router, routing::post};

use crate::{features::withdraw::controller::withdraw, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", post(withdraw))
}
