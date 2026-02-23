use crate::validation::address::validate_evm_address;
use hyperliquid::services::PermitSignature;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct HyperliquidUserPath {
    #[validate(custom(function = "validate_evm_address"))]
    pub user_evm_address: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct HyperliquidDepositRequest {
    pub amount: String,
    pub permit: PermitSignature,
}
