use core::types::PlatformsFundingRate;

use axum::{Json, extract::State};
use db::{
    crud::{insert_points, insert_user, insert_wallet},
    types::{Points, User, Wallet},
};
use sqlx::types::chrono;
use validator::Validate;

use crate::{
    error::AppError,
    types::{AppState, CreateUserPayload, SuccessResponse},
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
) -> Result<Json<SuccessResponse>, AppError> {
    // Validate the payload
    payload.validate()?;

    let timestamp = chrono::Utc::now();

    let wallet = Wallet {
        id: uuid::Uuid::new_v4(),
        turnkey_evm_address: None,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let wallet_id = insert_wallet(state.db.clone(), wallet).await?;
    let user_referral_code = payload.email.split("@").next().unwrap().to_string();

    let user = User {
        id: uuid::Uuid::new_v4(),
        email: Some(payload.email),
        connected_evm_address: payload.connected_evm_address,
        connected_solana_address: payload.connected_solana_address,
        referral_code: user_referral_code.clone(),
        referred_by: payload.referred_by,
        wallet_id,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let user_id = insert_user(state.db.clone(), user).await?;

    let points = Points {
        id: uuid::Uuid::new_v4(),
        user_id,
        points_to_added: 100,
        created_at: timestamp,
        updated_at: timestamp,
    };

    insert_points(state.db, points).await?;

    Ok(Json(SuccessResponse {
        message: "Successfully added to waitlist".to_string(),
        user_id: Some(user_id),
        user_referral_code: user_referral_code.clone(),
    }))
}
