use axum::{Router, routing::get};

use crate::{
    AppState,
    controller::user::{get_referral_count, get_total_points, get_total_users, get_user_position},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/total-users", get(get_total_users))
        .route("/total-points/{user_id}", get(get_total_points))
        .route("/referral-count/{referral_code}", get(get_referral_count))
        .route("/user-position/{user_id}", get(get_user_position))
}
