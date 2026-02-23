use crate::validation::validate_solana_address;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PacificaUserPath {
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PacificaDepositRequest {
    #[validate(range(min = 1, message = "Amount must be greater than 0"))]
    pub amount: u64,
}
