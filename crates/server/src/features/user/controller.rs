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

//TODO: fix validation after product is live
// pub async fn add_to_waitlist(
//     State(state): State<AppState>,
//     Json(payload): Json<CreateUserPayload>,
// ) -> Result<Json<WaitlistSuccessResponse>, AppError> {
//     // Validate the payload
//     payload.validate()?;

//     let timestamp = chrono::Utc::now().naive_utc();

//     let referred_by_user = match &payload.referred_by {
//         Some(code) => get_user_from_referral_code(state.db.clone(), code.clone())
//             .await
//             .ok(),
//         None => None,
//     };

//     let referred_by_user_id = match &referred_by_user {
//         Some(user) => Some(user.id),
//         None => None,
//     };

//     // Blocked user IDs
//     let blocked_user_ids = vec![
//         uuid::Uuid::parse_str("9b27257f-2f2a-41b8-9588-5a1a9c325e1c").unwrap(),
//         uuid::Uuid::parse_str("7d97d1f5-e5dd-4bbd-8031-929ee7c1fb10").unwrap(),
//         uuid::Uuid::parse_str("c378cda4-78ec-4e73-a3f7-894e14dd4094").unwrap(),
//         uuid::Uuid::parse_str("63ea1430-523a-430e-8292-e30902f3e2ac").unwrap(),
//         uuid::Uuid::parse_str("5c99748c-f0f5-476a-be77-07acab299e39").unwrap(),
//         uuid::Uuid::parse_str("ffb2e96c-557a-4ff2-9d91-b3fda52f07f6").unwrap(),
//         uuid::Uuid::parse_str("f74b601d-effd-40c4-9994-fc5f1cf7cde4").unwrap(),
//     ];

//     if let Some(ref_id) = referred_by_user_id {
//         // Check if user is blocked
//         if blocked_user_ids.contains(&ref_id) {
//             return Err(AppError::InternalServerError(String::from(
//                 "Too many requests",
//             )));
//         }

//         // Check referral limit (max 25 users per referral)
//         let referral_count = get_referral_count_from_user_id(state.db.clone(), ref_id).await?;
//         if referral_count >= 25 {
//             return Err(AppError::InternalServerError(String::from(
//                 "Referral limit reached",
//             )));
//         }
//     }

//     let points_to_add = if referred_by_user.is_some() { 125 } else { 100 };

//     let user_referral_code = nanoid::nanoid!();

//     let wallet = Wallet {
//         id: uuid::Uuid::new_v4(),
//         turnkey_evm_address: None,
//         created_at: timestamp,
//         updated_at: timestamp,
//     };

//     let user = User {
//         id: uuid::Uuid::new_v4(),
//         email: Some(payload.email),
//         connected_evm_address: payload.connected_evm_address,
//         connected_solana_address: payload.connected_solana_address,
//         referral_code: user_referral_code.clone(),
//         referred_by: referred_by_user_id.clone(),
//         wallet_id: wallet.id.clone(),
//         created_at: timestamp,
//         updated_at: timestamp,
//     };

//     let points = Points {
//         id: uuid::Uuid::new_v4(),
//         user_id: user.id,
//         points_to_add,
//         created_at: timestamp,
//         updated_at: timestamp,
//     };

//     // Insert wallet, user, and points in a single transaction
//     let user_id =
//         insert_user_with_wallet_and_points(state.db, wallet, user, points, referred_by_user)
//             .await?;

//     Ok(Json(WaitlistSuccessResponse {
//         message: "Successfully added to waitlist".to_string(),
//         user_id: Some(user_id),
//         user_referral_code,
//     }))
// }
