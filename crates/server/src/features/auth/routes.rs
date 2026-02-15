use axum::{Router, routing::post};

use crate::{
    AppState,
    features::auth::controller::{create_challenge, login},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/challenge", post(create_challenge))
        .route("/login", post(login))
}
