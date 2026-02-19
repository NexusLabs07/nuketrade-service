use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    pub time: u64,
    pub amount: String,
    pub destination: String, //TODO: this needs to be user's own address on the arbitrum chain
}

#[derive(Serialize, Deserialize)]
pub struct WithdrawResponse {
    action: WithdrawAction,
    #[serde(rename = "typedData")]
    typed_data: Value,
    nonce: u64,
    endpoint: String,
}

pub fn create_withdraw_typed_data(destination: &str, amount: &str, time: u64) -> Value {
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
    destination_address: Option<String>,
    amount: String,
) -> Result<WithdrawResponse, WithdrawError> {
    //TODO: verify the amount is greater than 0 and greater than user balance
    // TODO: verify that the destination address is evm compatible

    if destination_address.is_none() {
        return Err(WithdrawError::MissingDestinationAddress);
    }

    let destination_address = destination_address.unwrap();
    let nonce = 0; //TODO: need to get current time in seconds

    let typed_data = create_withdraw_typed_data(&destination_address, &amount, nonce);

    let withdraw_action = WithdrawAction {
        withdraw_type: "withdraw".to_string(),
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
        endpoint: String::from("https://api.hyperliquid.com/withdraw"),
    })
}
