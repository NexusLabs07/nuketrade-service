use serde::{Deserialize, Serialize};
use validator::Validate;

/// Claims extracted from Google's ID token (RS256, verified against Google JWKS).
#[derive(Debug, Deserialize)]
pub struct GoogleIdClaims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub aud: String,
    pub iss: String,
    pub exp: u64,
    pub iat: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub suborg_id: String,
    pub user_id: String,
    pub wallet_id: String,
    pub evm_address: String,
    pub solana_address: String,
    #[serde(alias = "issued_at")]
    pub iat: u64,
    #[serde(alias = "expiry")]
    pub exp: u64,
}

/// Unified login request — send `idToken` for Google, or omit it for wallet-only sign-in.
/// Turnkey fields (`suborgId`, `message`, `signature`) are always required.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// Google OAuth ID token; omit or leave empty for wallet sign-in.
    #[serde(default)]
    pub id_token: Option<String>,

    #[validate(length(min = 1, message = "suborgId must not be empty"))]
    pub suborg_id: String,
    #[validate(length(min = 1, message = "message must not be empty"))]
    pub message: String,
    #[validate(length(min = 1, message = "signature must not be empty"))]
    pub signature: String,

    pub access_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub expiry: u64,
}
