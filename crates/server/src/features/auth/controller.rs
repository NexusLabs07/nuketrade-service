use axum::{Json, extract::State};
use validator::Validate;

use crate::{error::AppError, state::AppState};

use super::types::{LoginRequest, LoginResponse};

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    payload.validate()?;

    let result = state
        .auth
        .login(payload.suborg_id.trim().to_string(), payload.message, payload.signature)
        .await?;

    state
        .auth
        .google_login(state.db.clone(), payload.id_token)
        .await?;

    Ok(Json(LoginResponse {
        token: result.token,
        evm_address: result.evm_address,
        solana_address: result.solana_address,
        expires_at_unix: result.expires_at_unix,
    }))
}
