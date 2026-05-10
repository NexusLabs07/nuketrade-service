use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppState,
    features::automation::internal_controller::{list_due_intents, record_result},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/intents/due", get(list_due_intents))
        .route("/intents/{intentId}/result", post(record_result))
}
