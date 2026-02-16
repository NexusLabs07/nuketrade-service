use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub suborg_id: String,
    pub evm_address: String,
    pub solana_address: String,
    #[serde(alias = "issued_at")]
    pub iat: u64,
    #[serde(alias = "expiry")]
    pub exp: u64,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 128, message = "suborgId must not be empty"))]
    pub suborg_id: String,
    #[validate(length(min = 1, max = 2048, message = "message must not be empty"))]
    pub message: String,
    #[validate(length(min = 1, max = 180, message = "signature must not be empty"))]
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub evm_address: String,
    pub solana_address: String,
    pub expires_at_unix: u64,
}
