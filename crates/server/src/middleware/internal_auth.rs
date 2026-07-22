//! Service-auth middleware for internal automation endpoints.
//!
//! These endpoints (`/internal/automation/*`) are called by the external
//! Node executor and MUST NOT be reachable with a user JWT or by an
//! unauthenticated client. We require a static bearer token configured via
//! `AUTOMATION_INTERNAL_TOKEN` and compare it in constant time.

use axum::{
    body::Body,
    extract::State,
    http::{Request, header},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::AppState};

/// Constant-time compare two byte slices. Returns false if lengths differ.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verifies `Authorization: Bearer <token>` against
/// `state.config.automation_internal_token`. Returns 401 on any mismatch
/// or if the server has no token configured.
pub async fn require_internal_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let configured = state
        .config
        .automation_internal_token
        .as_deref()
        .ok_or_else(|| {
            AppError::unauthorised(
                "automation internal endpoints are not configured (set AUTOMATION_INTERNAL_TOKEN)",
            )
        })?;

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorised("missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorised("Authorization must be `Bearer <token>`"))?;

    if !constant_time_eq(token.as_bytes(), configured.as_bytes()) {
        return Err(AppError::unauthorised("invalid internal service token"));
    }

    Ok(next.run(request).await)
}
