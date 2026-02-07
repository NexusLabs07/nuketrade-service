use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppState,
    features::hedge::controller::{
        create_hedge_intent, get_hedge_intent_detail, get_next_action, list_user_hedge_intents,
        report_action_result,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_hedge_intent))
        .route("/{id}", get(get_hedge_intent_detail))
        .route("/{id}/next-action", get(get_next_action))
        .route("/{id}/action-result", post(report_action_result))
        .route("/user/{user_id}", get(list_user_hedge_intents))
}
