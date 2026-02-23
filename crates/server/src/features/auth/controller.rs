use axum::{Json, extract::State};

use crate::{error::AppError, extractors::ValidatedJson, state::AppState};

use super::types::{LoginRequest, LoginResponse};

pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    if let Some(expected) = &state.config.access_code {
        let provided = payload.access_code.as_deref().unwrap_or("");
        if provided != expected.as_str() {
            return Err(AppError::unauthorised("Invalid access code"));
        }
    }

    let verify_signature_result = state
        .auth
        .login(
            payload.suborg_id.trim().to_string(),
            payload.message,
            payload.signature,
        )
        .await?;

    let (wallet_id, user_id) = state
        .auth
        .google_login(
            state.db.clone(),
            payload.id_token,
            verify_signature_result.evm_address.clone(),
            verify_signature_result.solana_address.clone(),
        )
        .await?;

    let (token, exp) = state.auth.issue_jwt(
        payload.suborg_id,
        user_id.to_string(),
        wallet_id.to_string(),
        verify_signature_result.evm_address.clone(),
        verify_signature_result.solana_address.clone(),
    )?;

    Ok(Json(LoginResponse { token: token }))
}
