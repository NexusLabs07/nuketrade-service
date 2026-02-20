use serde::{Deserialize, Serialize};

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
    pub evm_address: String,
    pub solana_address: String,
    #[serde(alias = "issued_at")]
    pub iat: u64,
    #[serde(alias = "expiry")]
    pub exp: u64,
}

/// Unified login request — send either `idToken` (Google) or the three Turnkey fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    // Google path
    pub id_token: String,

    // Turnkey path
    pub suborg_id: String,
    pub message: String,
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
