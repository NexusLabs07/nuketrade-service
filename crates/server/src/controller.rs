use core::types::PlatformsFundingRate;

use axum::{Json, extract::State};
use db::{
    crud::{insert_points, insert_user, insert_wallet},
    types::{Points, User, Wallet},
};
use sqlx::types::chrono;

use crate::types::{AppState, CreateUserPayload};

pub async fn root() -> &'static str {
    "Perpetual Aggregator Server is running."
}

pub async fn get_funding_rate(State(state): State<AppState>) -> Json<PlatformsFundingRate> {
    let snapshot = state.platforms_funding_rate.read().await;
    Json(snapshot.clone())
}

//TODO: add check for turnkey_evm_address to be "" for now
pub async fn add_to_waitlist(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> &'static str {
    let timestamp = chrono::Utc::now();

    let wallet = Wallet {
        id: uuid::Uuid::new_v4(),
        turnkey_evm_address: payload.turnkey_evm_address,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let wallet_id = insert_wallet(state.db.clone(), wallet).await.unwrap(); //TODO: handle failure

    let user = User {
        id: uuid::Uuid::new_v4(),
        email: payload.email,
        connected_evm_address: payload.connected_evm_address,
        connected_solana_address: payload.connected_solana_address,
        referral_code: nanoid::nanoid!(),
        referred_by: payload.referred_by,
        wallet_id,
        created_at: timestamp,
        updated_at: timestamp,
    };

    let user_id = insert_user(state.db.clone(), user).await.unwrap(); //TODO: handle unwrap

    let points = Points {
        id: uuid::Uuid::new_v4(),
        user_id,
        points_to_added: 100,
        created_at: timestamp,
        updated_at: timestamp,
    };

    insert_points(state.db, points).await.unwrap(); //TODO: handle unwrap

    "Ok"
}
