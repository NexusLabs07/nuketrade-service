use serde::{Deserialize, Serialize};
use sqlx::types::chrono;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct FundingRate {
    pub id: uuid::Uuid,
    pub platform: String,
    pub symbol: String,
    pub rate: f64,
    pub mark_px: f64,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct UserPayload {
    pub id: uuid::Uuid,
    pub email: Option<String>,
    pub connected_evm_address: Option<String>,
    pub connected_solana_address: Option<String>,
    pub referred_by: Option<uuid::Uuid>,
    pub turnkey_evm_address: String,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: uuid::Uuid,
    pub email: Option<String>,
    pub connected_evm_address: Option<String>,
    pub connected_solana_address: Option<String>,
    pub referral_code: String,
    pub referred_by: Option<uuid::Uuid>,
    pub wallet_id: uuid::Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct Wallet {
    pub id: uuid::Uuid,
    pub turnkey_evm_address: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct Points {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub points_to_add: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
