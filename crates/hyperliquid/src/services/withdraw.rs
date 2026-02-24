use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::HYPERLIQUID_HTTP_URL;

const MINIMUM_WITHDRAWAL_AMOUNT_USDC: f64 = 2.0;

#[derive(Debug, Serialize, Deserialize)]
pub enum WithdrawError {
    MissingDestinationAddress,
    InvalidAmount,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawAction {
    #[serde(rename = "type")]
    pub withdraw_type: String,
    #[serde(rename = "hyperliquidChain")]
    hyperliquid_chain: String,
    #[serde(rename = "signatureChainId")]
    signature_chain_id: String,
    pub time: u128,
    pub amount: String,
    pub destination: String,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawResponse {
    action: WithdrawAction,
    #[serde(rename = "typedData")]
    typed_data: Value,
    nonce: u128,
    endpoint: String,
}

pub fn create_withdraw_typed_data(destination: &str, amount: &str, time: u128) -> Value {
    json!({
        "types": {
            "EIP712Domain": [
                { "name": "name", "type": "string" },
                { "name": "version", "type": "string" },
                { "name": "chainId", "type": "uint256" },
                { "name": "verifyingContract", "type": "address" }
            ],
            "HyperliquidTransaction:Withdraw": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "destination", "type": "string" },
                { "name": "amount", "type": "string" },
                { "name": "time", "type": "uint64" }
            ]
        },
        "primaryType": "HyperliquidTransaction:Withdraw",
        "domain": {
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "chainId": 42161,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "message": {
            "hyperliquidChain": "Mainnet",
            "destination": destination,
            "amount": amount,
            "time": time
        }
    })
}

//TODO: add idempotency key to prevent double withdrawals while waiting
pub async fn withdraw(
    destination_address: String,
    amount: String,
) -> Result<WithdrawResponse, WithdrawError> {
    //TODO: verify the amount is greater than user balance
    // TODO: verify that the destination address is evm compatible

    let parsed_amount = amount
        .parse::<f64>()
        .map_err(|_| WithdrawError::InvalidAmount)?;

    if !parsed_amount.is_finite() || parsed_amount <= MINIMUM_WITHDRAWAL_AMOUNT_USDC {
        return Err(WithdrawError::InvalidAmount);
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let typed_data = create_withdraw_typed_data(&destination_address, &amount, nonce);

    let withdraw_action = WithdrawAction {
        withdraw_type: "withdraw3".to_string(),
        hyperliquid_chain: "Mainnet".to_string(),
        signature_chain_id: "0xa4b1".to_string(),
        time: nonce,
        amount: amount,
        destination: destination_address,
    };

    Ok(WithdrawResponse {
        action: withdraw_action,
        typed_data,
        nonce,
        endpoint: format!("{}/exchange", HYPERLIQUID_HTTP_URL),
    })
}
