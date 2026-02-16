use axum::{Router, routing::post};

use crate::{AppState, features::auth::controller::login};

pub fn routes() -> Router<AppState> {
    Router::new().route("/login", post(login))
}
