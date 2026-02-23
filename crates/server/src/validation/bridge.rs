use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    sol,
};
use bridge::MIN_BRIDGE_AMOUNT;
use perp_core::{Chain, chains::get_usdc_address, has_sufficient_balance};
use validator::ValidationError;

use crate::features::bridge::controller::QuotePayload;

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

pub fn validate_destination_usdc_address(
    payload: &QuotePayload,
) -> Result<(), validator::ValidationError> {
    let usdc_address = get_usdc_address(payload.destination_chain_id);

    if usdc_address.is_none() {
        return Err(validator::ValidationError::new("invalid_chain_id"));
    }

    Ok(())
}

/// Validates that the user has sufficient USDC balance on Base chain
pub async fn validate_balance(
    evm_address: String,
    amount: &String,
    base_rpc_url: &String,
) -> Result<(), validator::ValidationError> {
    let user_address: Address = evm_address.parse().map_err(|_| {
        let mut err = ValidationError::new("invalid_user_address");
        err.message = Some("Invalid user address format".into());
        err
    })?;

    let amount: U256 = amount.parse().map_err(|_| {
        let mut err = ValidationError::new("invalid_amount");
        err.message = Some("Amount must be a valid numeric string".into());
        err
    })?;

    if amount < U256::from(MIN_BRIDGE_AMOUNT) {
        let mut err = ValidationError::new("amount_less_than_10_usdc");
        err.message = Some("Amount must be at least 10 USDC (10000000)".into());
        return Err(err);
    }

    let usdc_address: Address = Chain::BASE.usdc_address.parse().map_err(|_| {
        let mut err = ValidationError::new("invalid_usdc_address");
        err.message = Some("Invalid USDC contract address".into());
        err
    })?;

    let provider = ProviderBuilder::new().connect_http(base_rpc_url.parse().unwrap());

    let usdc = IERC20::new(usdc_address, provider);

    let balance = usdc.balanceOf(user_address).call().await.map_err(|e| {
        let mut err = ValidationError::new("rpc_error");
        err.message = Some(format!("Failed to fetch balance from RPC: {e}").into());
        err
    })?;

    if !has_sufficient_balance(&balance, &amount) {
        let mut err = validator::ValidationError::new("insufficient_balance");
        err.message =
            Some(format!("Insufficient USDC: required {amount}, available {balance}").into());
        return Err(err);
    }

    Ok(())
}
