use axum::{Json, extract::State};

use crate::{error::AppError, extractors::ValidatedJson, state::AppState};

use super::types::{LoginRequest, LoginResponse};

pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
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
