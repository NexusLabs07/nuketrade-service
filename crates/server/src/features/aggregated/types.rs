use crate::validation::address::{
    validate_evm_address, validate_solana_address, validate_timeframe,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct MergedPositionsParams {
    #[validate(custom(function = "validate_evm_address"))]
    pub user_evm_address: String,
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChartParams {
    #[validate(custom(function = "validate_timeframe"))]
    pub timeframe: String,
}
