use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::AppState};

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
