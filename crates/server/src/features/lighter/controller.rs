use axum::{Extension, Json, extract::State};

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedJson, ValidatedPath},
    features::{
        auth::types::AuthClaims,
        pacifica::controller::{
            PacificaDepositRequest, PacificaUserPath,
            bridge_to_pacifica as pacifica_bridge_to_pacifica,
            get_user_open_positions as pacifica_get_user_open_positions,
        },
    },
    types::OpenPositionsResponse,
};

pub async fn get_user_open_positions(
    params: ValidatedPath<PacificaUserPath>,
    state: State<AppState>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    pacifica_get_user_open_positions(params, state).await
}

pub async fn bridge_to_pacifica(
    claims: Extension<AuthClaims>,
    state: State<AppState>,
    payload: ValidatedJson<PacificaDepositRequest>,
) -> Result<Json<String>, AppError> {
    pacifica_bridge_to_pacifica(claims, state, payload).await
}
