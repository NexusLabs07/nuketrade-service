use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::AppState};

/// Method-agnostic auth middleware — verifies the JWT and injects `AuthClaims` for any HTTP method.
/// Use this on routers that include GET endpoints requiring authentication.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    use crate::features::auth::types::AuthClaims;

    // Skip if claims were already injected by an outer middleware (e.g. require_post_auth).
    if request.extensions().get::<AuthClaims>().is_some() {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorised("missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorised("Authorization must be `Bearer <token>`"))?;

    let claims = state.auth.verify_token(token)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub async fn require_post_auth(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    if request.method() != Method::POST {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if path == "/auth" || path.starts_with("/auth/") {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorised("missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorised("Authorization must be `Bearer <token>`"))?;

    let claims = state.auth.verify_token(token)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        http::StatusCode,
        middleware as axum_mw,
        routing::{get, post},
    };
    use perp_core::{SevenDayApr, config::Config};
    use serde_json::Value;
    use sqlx::postgres::PgPoolOptions;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::watch;
    use tower::ServiceExt;

    use crate::{
        features::auth::{services::AuthService, types::AuthClaims},
        types::FeedSnapshot,
    };

    const JWT_SECRET: &str = "test-middleware-secret";

    fn test_config() -> Config {
        Config {
            db_url: "postgres://localhost/fake".into(),
            solana_rpc_url: "https://fake".into(),
            arbitrum_rpc_url: "https://fake".into(),
            base_rpc_url: "https://fake".into(),
            server_host: "127.0.0.1".into(),
            server_port: 9999,
            cors_allowed_origins: vec![],
            evm_fee_payer_private_key: "a".repeat(64),
            solana_fee_payer_private_key: "fake".into(),
            relay_api_key: "fake-rel".into(),
            turnkey_api_base_url: "https://fake".into(),
            turnkey_parent_org_id: "fake-org".into(),
            turnkey_api_public_key: "fake-pub".into(),
            turnkey_api_private_key: "a".repeat(64),
            auth_jwt_secret: JWT_SECRET.into(),
            auth_jwt_ttl_days: 1,
            google_client_id: String::from("fake_client"),
            access_code: Some(String::from("ABCD")),
        }
    }

    fn test_app_state() -> AppState {
        let config = test_config();
        let auth = AuthService::from_config(&config).unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.db_url)
            .unwrap();
        let (feed_tx, feed_rx) = watch::channel(Arc::new(FeedSnapshot {
            by_symbol: HashMap::new(),
            formatted: vec![],
        }));
        let _ = feed_tx; // keep sender alive
        let (apr_tx, apr_rx) = watch::channel(SevenDayApr {
            seven_day_avg_apr: HashMap::new(),
            seven_day_spread_apr: HashMap::new(),
        });
        let _ = apr_tx;

        AppState::new(config, Arc::new(pool), feed_rx, apr_rx, auth)
    }

    fn test_router(state: AppState) -> Router {
        Router::new()
            .route("/test", post(|| async { "ok" }))
            .route("/test", get(|| async { "ok-get" }))
            .route("/auth", post(|| async { "auth-endpoint" }))
            .route("/auth/login", post(|| async { "auth-login" }))
            .route(
                "/whoami",
                post(|claims: axum::Extension<AuthClaims>| async move {
                    Json(serde_json::json!({
                        "evm": claims.evm_address,
                        "sol": claims.solana_address,
                    }))
                }),
            )
            .layer(axum_mw::from_fn_with_state(
                state.clone(),
                require_post_auth,
            ))
            .with_state(state)
    }

    fn issue_test_token(_state: &AppState) -> String {
        // Use verify_token round-trip via the auth service internals.
        // We'll craft a valid JWT using the same secret.
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = AuthClaims {
            suborg_id: "test-sub".into(),
            user_id: "user-test".into(),
            wallet_id: "wallet-test".into(),
            evm_address: "0x1234".into(),
            solana_address: "SolAddr".into(),
            iat: now,
            exp: now + 3600,
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn get_request_passes_through_without_auth() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_to_auth_passes_through_without_token() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/auth")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_to_auth_subpath_passes_through() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/auth/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_without_auth_header_returns_401() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["message"], "missing Authorization header");
    }

    #[tokio::test]
    async fn post_with_non_bearer_header_returns_401() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/test")
                    .header(header::AUTHORIZATION, "Basic abc123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["message"], "Authorization must be `Bearer <token>`");
    }

    #[tokio::test]
    async fn post_with_invalid_token_returns_401() {
        let state = test_app_state();
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/test")
                    .header(header::AUTHORIZATION, "Bearer invalid.token.here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn post_with_valid_token_passes_through_and_injects_claims() {
        let state = test_app_state();
        let token = issue_test_token(&state);
        let app = test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/whoami")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["evm"], "0x1234");
        assert_eq!(json["sol"], "SolAddr");
    }
}
