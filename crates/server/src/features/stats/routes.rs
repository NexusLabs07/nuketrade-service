use axum::routing::get;

use crate::features::stats::controller::get_total_volume;

pub fn routes() -> axum::Router<crate::AppState> {
    axum::Router::new().route("/total-volume", get(get_total_volume))
}
