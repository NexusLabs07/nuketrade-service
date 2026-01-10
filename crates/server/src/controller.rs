use core::types::PlatformsFundingRate;

use axum::{
    Json,
    extract::{Path, State},
};
use db::{
    crud::{
        get_referral_count_from_user_id, get_user_from_referral_code,
        insert_user_with_wallet_and_points,
    },
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

    // Blocked user IDs
    let blocked_user_ids = vec![
        uuid::Uuid::parse_str("9b27257f-2f2a-41b8-9588-5a1a9c325e1c").unwrap(),
        uuid::Uuid::parse_str("7d97d1f5-e5dd-4bbd-8031-929ee7c1fb10").unwrap(),
        uuid::Uuid::parse_str("c378cda4-78ec-4e73-a3f7-894e14dd4094").unwrap(),
        uuid::Uuid::parse_str("63ea1430-523a-430e-8292-e30902f3e2ac").unwrap(),
        uuid::Uuid::parse_str("5c99748c-f0f5-476a-be77-07acab299e39").unwrap(),
        uuid::Uuid::parse_str("ffb2e96c-557a-4ff2-9d91-b3fda52f07f6").unwrap(),
        uuid::Uuid::parse_str("f74b601d-effd-40c4-9994-fc5f1cf7cde4").unwrap(),
    ];

    if let Some(ref_id) = referred_by_user_id {
        // Check if user is blocked
        if blocked_user_ids.contains(&ref_id) {
            return Err(AppError::InternalServerError(String::from(
                "Too many requests",
            )));
        }

        // Check referral limit (max 25 users per referral)
        let referral_count = get_referral_count_from_user_id(state.db.clone(), ref_id).await?;
        if referral_count >= 25 {
            return Err(AppError::InternalServerError(String::from(
                "Referral limit reached",
            )));
        }
    }

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
