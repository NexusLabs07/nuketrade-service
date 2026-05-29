use axum::{Json, extract::State};

use crate::{
    AppState,
    error::AppError,
    features::stats::services::{TotalVolumeResponse, calculate_total_volume},
};

pub async fn get_total_volume(
    State(state): State<AppState>,
) -> Result<Json<TotalVolumeResponse>, AppError> {
    let response = calculate_total_volume(state.db).await?;
    Ok(Json(response))
}
