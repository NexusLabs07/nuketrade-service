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
    // Temporary local-testing bypass: when set, internal automation endpoints
    // require no auth at all. Must be present in the **Rust process** env (e.g.
    // repo-root `.env` loaded by `bin/executor`, or exported before `cargo run`).
    // Setting this only in Node's `.env` has no effect on Rust.
    // This escape hatch exists solely for local development. A release build
    // must never expose internal endpoints without authentication, even when a
    // deployment environment accidentally sets DISABLE_AUTOMATION_AUTH.
    let bypass_requested = std::env::var("DISABLE_AUTOMATION_AUTH")
        .ok()
        .is_some_and(|value| {
            let value = value.trim();
            value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
        });

    if bypass_requested {
        let local_environment = std::env::var("APP_ENV").ok().is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "development" | "dev" | "test"
            )
        });

        if cfg!(debug_assertions) && local_environment {
            log::warn!(
                "DISABLE_AUTOMATION_AUTH is enabled for a local debug build; \
                 internal automation endpoints are unauthenticated"
            );
            return Ok(next.run(request).await);
        }

        log::error!("Ignoring DISABLE_AUTOMATION_AUTH outside a local debug environment");

        return Err(AppError::unauthorised(
            "automation internal authentication cannot be disabled in this environment",
        ));
    }

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
