use axum::Router;
use axum::middleware as axum__middleware;
use axum::routing::get;

// [automation disabled] incomplete — automation feature commented out for now
use crate::features::feed;
use crate::middleware::internal_auth::require_internal_auth;
use crate::state::AppState;

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub fn create_app(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        // [automation disabled] incomplete — automation internal API not mounted
        // .nest(
        //     "/internal/automation",
        //     automation::internal_routes::routes().layer(axum__middleware::from_fn_with_state(
        //         app_state.clone(),
        //         require_internal_auth,
        //     )),
        // )
        .nest(
            "/internal/feed",
            Router::new()
                .route("/snapshot", get(feed::get_feed_snapshot))
                .layer(axum__middleware::from_fn_with_state(
                    app_state.clone(),
                    require_internal_auth,
                )),
        )
        .with_state(app_state)
}
