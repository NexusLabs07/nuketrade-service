// ============================= Types =============================

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct HedgeIntent {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub asset: String,
    pub exchange_a: String,
    pub exchange_b: String,
    pub margin_usd: f64,
    pub leverage: f64,
    pub evm_address: String,
    pub solana_address: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct HedgeLeg {
    pub id: Uuid,
    pub hedge_intent_id: Uuid,
    pub exchange: String,
    pub chain: String,
    pub target_amount_usd: f64,
    pub funded_amount_usd: f64,
    pub status: String,
    pub retry_count: i16,
    pub last_error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// USDC already inside the protocol's margin account (queried on CREATED→FUNDING).
    pub existing_margin_usd: f64,
    /// USDC sitting on the destination chain but not deposited (queried on CREATED→FUNDING).
    pub existing_onchain_usd: f64,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct TxReference {
    pub id: Uuid,
    pub hedge_leg_id: Uuid,
    pub action: String,
    pub tx_hash: Option<String>,
    pub chain: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

/// Payload for creating a new hedge intent (no timestamps needed, DB defaults handle them).
#[derive(Debug, Clone)]
pub struct NewHedgeIntent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub asset: String,
    pub exchange_a: String,
    pub exchange_b: String,
    pub margin_usd: f64,
    pub leverage: f64,
    pub evm_address: String,
    pub solana_address: String,
}

/// Payload for creating a new hedge leg.
#[derive(Debug, Clone)]
pub struct NewHedgeLeg {
    pub id: Uuid,
    pub hedge_intent_id: Uuid,
    pub exchange: String,
    pub chain: i64,
    pub target_amount_usd: f64,
}

/// Payload for creating a new tx reference.
#[derive(Debug, Clone)]
pub struct NewTxReference {
    pub id: Uuid,
    pub hedge_leg_id: Uuid,
    pub action: String,
    pub tx_hash: Option<String>,
    pub chain: String,
    pub status: String,
}
