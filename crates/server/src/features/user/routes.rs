use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppState,
    features::user::controller::{
        get_pacifica_claim_status, get_referral_count, get_total_points, get_total_users,
        get_user_position, mark_pacifica_claim,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/total-users", get(get_total_users))
        .route("/total-points/{user_id}", get(get_total_points))
        .route("/claim/pacifica", post(mark_pacifica_claim))
        .route(
            "/claim-status/pacifica/{user_id}",
            get(get_pacifica_claim_status),
        )
        .route("/referral-count/{referral_code}", get(get_referral_count))
        .route("/user-position/{user_id}", get(get_user_position))
}
