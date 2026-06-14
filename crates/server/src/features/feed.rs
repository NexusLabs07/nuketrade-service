//! Internal feed endpoint consumed by the TypeScript API server.

use axum::{Json, extract::State};

use crate::{state::AppState, types::FeedSnapshotResponse};

/// Internal: full feed snapshot (unfiltered) plus 7d APR, consumed by the
/// TypeScript API server.
pub async fn get_feed_snapshot(State(state): State<AppState>) -> Json<FeedSnapshotResponse> {
    let snapshot = state.feed.borrow().clone();
    let seven_day_apr = state.seven_day_apr.borrow().clone();

    Json(FeedSnapshotResponse {
        feed: snapshot.formatted.clone(),
        seven_day_apr,
    })
}
