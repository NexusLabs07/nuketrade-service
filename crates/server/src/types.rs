use core::types::PlatformsFundingRate;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Arc<PgPool>,
    pub platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateUserPayload {
    pub email: Option<String>,
    pub connected_evm_address: Option<String>,
    pub connected_solana_address: Option<String>,
    pub referred_by: Option<uuid::Uuid>,
    pub turnkey_evm_address: String,
}
