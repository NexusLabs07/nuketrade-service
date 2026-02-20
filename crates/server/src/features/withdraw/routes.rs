use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppState,
    features::withdraw::controller::{
        bridge, create_withdraw_transaction, create_withdrawal_intent, get_next_action,
        get_withdrawal_intent_detail, list_user_withdrawal_intents, report_action_result,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/create-intent", post(create_withdrawal_intent))
        .route("/transaction", post(create_withdraw_transaction))
        .route("/bridge", post(bridge))
        .route("/{id}", get(get_withdrawal_intent_detail))
        .route("/{id}/next-action", get(get_next_action))
        .route("/{id}/action-result", post(report_action_result))
        .route("/user/{user_id}", get(list_user_withdrawal_intents))
}
