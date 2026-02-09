use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{AppState, error::AppError, middleware::user::validate_addresses_are_null};

#[derive(Serialize)]
pub struct WaitlistSuccessResponse {
    pub message: String,
    pub user_id: Option<uuid::Uuid>,
    pub user_referral_code: String,
}

#[derive(Serialize)]
pub struct TotalUsersSuccessResponse {
    pub total_users: i64,
}

#[derive(Serialize)]
pub struct TotalPointsSuccessResponse {
    pub total_points: i32,
}

#[derive(Serialize)]
pub struct UserPositionSuccessResponse {
    pub position: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_addresses_are_null"))]
pub struct CreateUserPayload {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    pub connected_evm_address: Option<String>,

    pub connected_solana_address: Option<String>,

    pub referred_by: Option<String>,

    pub turnkey_evm_address: Option<String>,
}

pub async fn get_total_users(
    State(state): State<AppState>,
) -> Result<Json<TotalUsersSuccessResponse>, AppError> {
    let total_users = db::user::get_total_users(state.db).await?;

    Ok(Json(TotalUsersSuccessResponse { total_users }))
}

pub async fn get_total_points(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<TotalPointsSuccessResponse>, AppError> {
    let total_points = db::points::get_total_points(state.db, user_id).await?;

    Ok(Json(TotalPointsSuccessResponse { total_points }))
}

pub async fn get_referral_count(
    State(state): State<AppState>,
    Path(referral_code): Path<String>,
) -> Result<Json<i32>, AppError> {
    let count = db::user::get_referral_count_from_referral_code(state.db, referral_code).await?;

    Ok(Json(count))
}

pub async fn get_user_position(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<UserPositionSuccessResponse>, AppError> {
    let position = db::user::get_user_position(state.db, user_id).await?;

    Ok(Json(UserPositionSuccessResponse { position }))
}
