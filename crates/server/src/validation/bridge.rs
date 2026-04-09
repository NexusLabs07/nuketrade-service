use bridge::MIN_BRIDGE_AMOUNT;
use perp_core::{Chain, chains::get_usdc_address};
use validator::ValidationError;

use crate::features::bridge::controller::QuotePayload;

pub fn validate_destination_usdc_address(
    payload: &QuotePayload,
) -> Result<(), validator::ValidationError> {
    let usdc_address = get_usdc_address(payload.destination_chain_id);

    if usdc_address.is_none() {
        return Err(validator::ValidationError::new("invalid_chain_id"));
    }

    Ok(())
}

/// Validates that the user has sufficient USDC balance on Solana.
pub async fn validate_solana_balance(
    solana_address: &str,
    amount: &str,
    solana_rpc_url: &str,
) -> Result<(), ValidationError> {
    let amount_u64: u64 = amount.parse().map_err(|_| {
        let mut err = ValidationError::new("invalid_amount");
        err.message = Some("Amount must be a valid numeric string".into());
        err
    })?;

    if amount_u64 < MIN_BRIDGE_AMOUNT {
        let mut err = ValidationError::new("amount_less_than_min");
        err.message = Some("Amount must be at least 2 USDC (2000000)".into());
        return Err(err);
    }

    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            solana_address,
            { "mint": Chain::SOLANA.usdc_address },
            { "encoding": "jsonParsed" }
        ]
    });

    let response = client
        .post(solana_rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let mut err = ValidationError::new("rpc_error");
            err.message = Some(format!("Failed to query Solana RPC: {e}").into());
            err
        })?;

    let data: serde_json::Value = response.json().await.map_err(|e| {
        let mut err = ValidationError::new("rpc_error");
        err.message = Some(format!("Failed to parse Solana RPC response: {e}").into());
        err
    })?;

    // Sum USDC balance across all token accounts for this mint.
    let balance_raw: u64 = data["result"]["value"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|account| {
            let info = account["account"]["data"]["parsed"]["info"].as_object()?;
            let amount_str = info["tokenAmount"].as_object()?.get("amount")?.as_str()?;
            amount_str.parse::<u64>().ok()
        })
        .sum();

    if balance_raw < amount_u64 {
        let mut err = ValidationError::new("insufficient_balance");
        err.message = Some(
            format!(
                "Insufficient USDC: required {amount_u64}, available {balance_raw}"
            )
            .into(),
        );
        return Err(err);
    }

    Ok(())
}
