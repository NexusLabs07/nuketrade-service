use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    sol,
};
use perp_core::{Chain, chains::get_usdc_address};

use crate::controller::bridge::QuotePayload;

const BASE_RPC_URL: &str = "https://mainnet.base.org";

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
    let user_address: Address = payload
        .user
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_user_address"))?;

    let amount: U256 = payload
        .amount
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_amount"))?;

    if amount < U256::from(10_000_000) {
        return Err(validator::ValidationError::new("amount_less_than_10_usdc"));
    }

    let usdc_address: Address = Chain::BASE
        .usdc_address
        .parse()
        .map_err(|_| validator::ValidationError::new("invalid_usdc_address"))?;

    let provider = ProviderBuilder::new().connect_http(BASE_RPC_URL.parse().unwrap());

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
