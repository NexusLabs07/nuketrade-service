use core::types::PlatformsFundingRate;

use axum::{
    Json,
    extract::{Path, State},
};
use db::{
    crud::{get_user_from_referral_code, insert_user_with_wallet_and_points},
    types::{Points, User, Wallet},
};
use sqlx::types::chrono;
use validator::Validate;

use crate::{
    error::AppError,
    types::{
        AppState, CreateUserPayload, TotalPointsSuccessResponse, TotalUsersSuccessResponse,
        WaitlistSuccessResponse,
    },
};

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub async fn get_funding_rate(State(state): State<AppState>) -> Json<PlatformsFundingRate> {
    let snapshot = state.platforms_funding_rate.read().await;
    Json(snapshot.clone())
}

//TODO: fix validation after product is live
pub async fn add_to_waitlist(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<WaitlistSuccessResponse>, AppError> {
    // Validate the payload
    payload.validate()?;

    let timestamp = chrono::Utc::now().naive_utc();

    let referred_by_user = match &payload.referred_by {
        Some(code) => get_user_from_referral_code(state.db.clone(), code.clone())
            .await
            .ok(),
        None => None,
    };

    let referred_by_user_id = match &referred_by_user {
        Some(user) => Some(user.id),
        None => None,
    };

    let points_to_add = if referred_by_user.is_some() { 125 } else { 100 };

    let user_referral_code = nanoid::nanoid!();

    let wallet = Wallet {
        id: uuid::Uuid::new_v4(),
        turnkey_evm_address: None,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let user = User {
        id: uuid::Uuid::new_v4(),
        email: Some(payload.email),
        connected_evm_address: payload.connected_evm_address,
        connected_solana_address: payload.connected_solana_address,
        referral_code: user_referral_code.clone(),
        referred_by: referred_by_user_id.clone(),
        wallet_id: wallet.id.clone(),
        created_at: timestamp,
        updated_at: timestamp,
    };

    let points = Points {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        points_to_add,
        created_at: timestamp,
        updated_at: timestamp,
    };

    // Insert wallet, user, and points in a single transaction
    let user_id =
        insert_user_with_wallet_and_points(state.db, wallet, user, points, referred_by_user)
            .await?;

    Ok(Json(WaitlistSuccessResponse {
        message: "Successfully added to waitlist".to_string(),
        user_id: Some(user_id),
        user_referral_code,
    }))
}

pub async fn get_total_users(
    State(state): State<AppState>,
) -> Result<Json<TotalUsersSuccessResponse>, AppError> {
    let total_users = db::crud::get_total_users(state.db).await?;

    Ok(Json(TotalUsersSuccessResponse { total_users }))
}

pub async fn get_total_points(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<TotalPointsSuccessResponse>, AppError> {
    let total_points = db::crud::get_total_points(state.db, user_id).await?;

    Ok(Json(TotalPointsSuccessResponse { total_points }))
}

pub async fn get_referral_count(
    State(state): State<AppState>,
    Path(referral_code): Path<String>,
) -> Result<Json<i32>, AppError> {
    let count = db::crud::get_referral_count_from_referral_code(state.db, referral_code).await?;

    Ok(Json(count))
}

pub async fn get_user_position(
    State(state): State<AppState>,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<Json<crate::types::UserPositionSuccessResponse>, AppError> {
    let position = db::crud::get_user_position(state.db, user_id).await?;

    Ok(Json(crate::types::UserPositionSuccessResponse { position }))
}
