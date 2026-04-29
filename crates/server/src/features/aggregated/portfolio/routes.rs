use axum::routing::get;

use crate::{
    AppState,
    features::aggregated::portfolio::controller::{
        get_exchanges, get_performance, get_pnl_chart,
    },
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/performance/{user_evm_address}/{user_solana_address}",
            get(get_performance),
        )
        .route(
            "/pnl-chart/{user_evm_address}/{user_solana_address}",
            get(get_pnl_chart),
        )
        .route(
            "/exchanges/{user_evm_address}/{user_solana_address}",
            get(get_exchanges),
        )
}
