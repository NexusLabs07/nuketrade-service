use axum::routing::get;

use crate::{
    AppState,
    controller::aggregated::{get_merged_open_positions, get_tokens_funding},
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/open-positions/{user_evm_address}/{user_solana_address}",
            get(get_merged_open_positions),
        )
        .route("/tokens/funding", get(get_tokens_funding))
}
