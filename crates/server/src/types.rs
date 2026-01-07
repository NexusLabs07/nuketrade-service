use core::types::PlatformsFundingRate;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::RwLock;
use validator::Validate;

#[derive(Clone, Debug)]
pub struct AppState {
    pub db: Arc<PgPool>,
    pub platforms_funding_rate: Arc<RwLock<PlatformsFundingRate>>,
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

fn validate_addresses_are_null(
    payload: &CreateUserPayload,
) -> Result<(), validator::ValidationError> {
    if payload.connected_evm_address.is_some() {
        return Err(validator::ValidationError::new(
            "connected_evm_address must be null for now",
        ));
    }
    if payload.connected_solana_address.is_some() {
        return Err(validator::ValidationError::new(
            "connected_solana_address must be null for now",
        ));
    }
    if payload.turnkey_evm_address.is_some() {
        return Err(validator::ValidationError::new(
            "turnkey_evm_address must be null for now",
        ));
    }
    Ok(())
}

fn validate_evm_address(address: &str) -> Result<(), validator::ValidationError> {
    if !address.starts_with("0x") {
        return Err(validator::ValidationError::new("must_start_with_0x"));
    }

    if address.len() != 42 {
        return Err(validator::ValidationError::new("invalid_length"));
    }

    let hex_part = &address[2..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(validator::ValidationError::new("invalid_hex"));
    }

    Ok(())
}

fn validate_solana_address(address: &str) -> Result<(), validator::ValidationError> {
    if !address.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(validator::ValidationError::new("invalid_base58"));
    }

    Ok(())
}

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
