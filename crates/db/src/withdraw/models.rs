use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct WithdrawalIntent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exchange: String,
    pub amount_usd: f64,
    pub evm_address: String,
    pub recipient: String,
    pub destination_chain_id: i32,
    pub status: String,
    pub retry_count: i16,
    pub last_error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct WithdrawalStep {
    pub id: Uuid,
    pub withdrawal_intent_id: Uuid,
    pub step: String,
    pub tx_hash: Option<String>,
    pub chain_id: Option<i32>,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct NewWithdrawalIntent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exchange: String,
    pub amount_usd: f64,
    pub evm_address: String,
    pub recipient: String,
    pub destination_chain_id: i32,
}

#[derive(Debug, Clone)]
pub struct NewWithdrawalStep {
    pub id: Uuid,
    pub withdrawal_intent_id: Uuid,
    pub step: String,
    pub chain_id: Option<i32>,
}
