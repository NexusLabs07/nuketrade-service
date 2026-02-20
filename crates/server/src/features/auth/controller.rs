use axum::{Json, extract::State};

use crate::{error::AppError, state::AppState};

use super::types::{LoginRequest, LoginResponse};

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    //Verify if the details are correct first
    let suborg_id = payload.suborg_id;
    let message = payload.message;
    let signature = payload.signature;

    let result = state
        .auth
        .login(suborg_id.trim().to_string(), message, signature)
        .await?;

    //then verify if the google token is correct and save the details of user if doesn't exist
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
