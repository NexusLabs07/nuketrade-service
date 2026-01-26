use axum::routing::get;

use crate::{
    AppState,
    controller::aggregated::{get_live_market_feed, get_merged_open_positions, get_token_chart},
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/open-positions/{user_evm_address}/{user_solana_address}",
            get(get_merged_open_positions),
        )
        .route("/chart/{symbol}", get(get_token_chart))
        .route("/live/market-feed", get(get_live_market_feed))
}
