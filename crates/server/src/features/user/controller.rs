use axum::{
    Json,
    extract::{Path, State},
};
use db::user::queries;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    middleware::user::{validate_addresses_are_null, validate_evm_address, validate_solana_address},
};

#[derive(Serialize)]
pub struct WaitlistSuccessResponse {
    pub message: String,
    pub user_id: Option<uuid::Uuid>,
    pub user_referral_code: String,
}

#[derive(Serialize, Deserialize)]
pub struct PacificaClaimRequest {
    user_id: uuid::Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct PacificaClaimResponse {
    is_claimed: bool,
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

    #[validate(custom(function = "validate_evm_address"))]
    pub connected_evm_address: Option<String>,

    #[validate(custom(function = "validate_solana_address"))]
    pub connected_solana_address: Option<String>,

    pub referred_by: Option<String>,

    #[validate(custom(function = "validate_evm_address"))]
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

pub async fn mark_pacifica_claim(
    State(state): State<AppState>,
    Json(payload): Json<PacificaClaimRequest>,
) -> Result<(), AppError> {
    if let Err(err) = queries::mark_pacifica_claim(state.db, payload.user_id).await {
        return Err(AppError::Database(err));
    };

    Ok(())
}

pub async fn get_pacifica_claim_status(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<PacificaClaimResponse>, AppError> {
    let user_id_uuid = uuid::Uuid::parse_str(&user_id).map_err(|_| AppError::Parse {
        field: String::from("uuid"),
        message: String::from("Failed to parse string into uuid"),
    })?;

    let result = queries::get_pacifica_claim_status(state.db, user_id_uuid).await?;

    Ok(Json(PacificaClaimResponse { is_claimed: result }))
}
