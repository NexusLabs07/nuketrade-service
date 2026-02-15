use axum::{Json, extract::State};
use validator::Validate;

use crate::{error::AppError, state::AppState};

use super::types::{CreateChallengeRequest, CreateChallengeResponse, LoginRequest, LoginResponse};

pub async fn create_challenge(
    State(state): State<AppState>,
    Json(payload): Json<CreateChallengeRequest>,
) -> Result<Json<CreateChallengeResponse>, AppError> {
    payload.validate()?;

    let (message, nonce, expires_at_unix) = state
        .auth
        .create_challenge(payload.suborg_id.trim().to_string())
        .await?;

    Ok(Json(CreateChallengeResponse {
        message,
        nonce,
        expires_at_unix,
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    payload.validate()?;

    let result = state
        .auth
        .login(
            payload.suborg_id.trim().to_string(),
            payload.message,
            payload.signature,
        )
        .await?;

    Ok(Json(LoginResponse {
        token: result.token,
        evm_address: result.evm_address,
        solana_address: result.solana_address,
        expires_at_unix: result.expires_at_unix,
    }))
}
