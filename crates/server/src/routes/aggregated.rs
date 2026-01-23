use axum::routing::get;

use crate::{AppState, controller::aggregated::get_merged_open_positions};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route(
        "/open-positions/{user_evm_address}/{user_solana_address}",
        get(get_merged_open_positions),
    )
}
