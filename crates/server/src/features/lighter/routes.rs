use axum::routing::{get, post};

use crate::{
    AppState,
    features::lighter::controller::{deposit, get_fee_payer, get_perp_metadata},
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/perp-metadata", get(get_perp_metadata))
        .route("/fee-payer", get(get_fee_payer))
        .route("/deposit", post(deposit))
}
