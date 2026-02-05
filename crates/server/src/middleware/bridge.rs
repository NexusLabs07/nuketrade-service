use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    sol,
};
use bridge::MIN_BRIDGE_AMOUNT;
use perp_core::{Chain, chains::get_usdc_address, config::Config};
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
pub async fn validate_balance(payload: &QuotePayload) -> Result<(), validator::ValidationError> {
    let base_rpc_url = Config::from_env()
        .map_err(|_| ValidationError::new("invalid base rpc url"))?
        .base_rpc_url;

    let user_address: Address = payload
        .user
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_user_address"))?;

    let amount: U256 = payload
        .amount
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_amount"))?;

    if amount < U256::from(MIN_BRIDGE_AMOUNT) {
        return Err(validator::ValidationError::new("amount_less_than_10_usdc"));
    }

    let usdc_address: Address = Chain::BASE
        .usdc_address
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_usdc_address"))?;

    let provider = ProviderBuilder::new().connect_http(base_rpc_url.parse().unwrap());

    let usdc = IERC20::new(usdc_address, provider);

    let balance = usdc
        .balanceOf(user_address)
        .call()
        .await
        .map_err(|_| validator::ValidationError::new("rpc_error"))?;

    if balance < amount {
        let mut err = validator::ValidationError::new("insufficient_balance");
        err.message = Some(
            format!(
                "Insufficient USDC: required {}, available {}",
                amount, balance
            )
            .into(),
        );
        return Err(err);
    }

    Ok(())
}
