//! Internal HTTP controllers for the Node executor.
//!
//! All handlers in this module are mounted behind
//! `crate::middleware::internal_auth::require_internal_auth` and require a
//! shared bearer token (`AUTOMATION_INTERNAL_TOKEN`). They are NOT meant
//! to be exposed to user JWTs or to the public internet.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppError,
    extractors::ValidatedJson,
    features::automation::services::{
        AutomationService, DueIntent, FALLBACK_LEASE_TTL_SEC, IntentResult, IntentResultAck,
    },
    state::AppState,
};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DueIntentsQuery {
    #[validate(range(min = 1, max = 200, message = "limit must be 1..=200"))]
    pub limit: Option<i64>,
}

/// `GET /internal/automation/intents/due?limit=N`
///
/// Returns up to N actionable intents and leases each one to the calling
/// worker (worker id from `X-Worker-Id` or a random per-request id).
pub async fn list_due_intents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<DueIntentsQuery>,
) -> Result<Json<Vec<DueIntent>>, AppError> {
    let limit = params.limit.unwrap_or(25).clamp(1, 200);
    let worker = headers
        .get("x-worker-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("worker-{}", Uuid::new_v4()));

    let lease_ttl = if state.config.automation_lease_ttl_sec > 0 {
        state.config.automation_lease_ttl_sec
    } else {
        FALLBACK_LEASE_TTL_SEC
    };

    let intents = AutomationService::fetch_due_intents(
        state.db,
        &state.feed,
        &state.seven_day_apr,
        &worker,
        limit,
        lease_ttl,
    )
    .await?;
    Ok(Json(intents))
}

/// `POST /internal/automation/intents/{intentId}/result`
pub async fn record_result(
    State(state): State<AppState>,
    Path(intent_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<IntentResult>,
) -> Result<Json<IntentResultAck>, AppError> {
    let ack = AutomationService::record_intent_result(state.db, intent_id, payload).await?;
    Ok(Json(ack))
}
