use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppState,
    features::automation::controller::{
        create_run, get_best_pair, get_config, get_run, get_run_actions, list_runs, pause_run,
        restart_run, resume_run, stop_run, upsert_config,
    },
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_config).put(upsert_config))
        .route("/best-pair", get(get_best_pair))
        .route("/runs", post(create_run).get(list_runs))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/pause", post(pause_run))
        .route("/runs/{id}/resume", post(resume_run))
        .route("/runs/{id}/stop", post(stop_run))
        .route("/runs/{id}/restart", post(restart_run))
        .route("/runs/{id}/actions", get(get_run_actions))
}
