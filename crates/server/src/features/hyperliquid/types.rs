use crate::validation::validate_evm_address;
use hyperliquid::ops::deposit::PermitSignature;
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
