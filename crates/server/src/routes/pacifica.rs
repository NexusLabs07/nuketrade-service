use axum::routing::get;

use crate::{AppState, controller::pacifica::get_user_open_positions};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route(
        "/open-positions/{user_solana_adresss}",
        get(get_user_open_positions),
    )
}
