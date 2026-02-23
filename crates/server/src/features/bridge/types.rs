use crate::validation::bridge::validate_destination_usdc_address;
use bridge::client::PermitRequest;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[validate(schema(function = "validate_destination_usdc_address"))]
pub struct QuotePayload {
    #[serde(rename = "destinationChainId")]
    pub destination_chain_id: u64,
    #[validate(length(min = 1, message = "Amount must not be empty"))]
    pub amount: String,
    #[serde(rename = "tradeType")]
    pub trade_type: String,
    #[serde(rename = "usePermit")]
    pub use_permit: bool,
    #[validate(length(min = 1, message = "Recipient must not be empty"))]
    pub recipient: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExecutePermitPayload {
    #[validate(length(min = 1, message = "Signature must not be empty"))]
    pub signature: String,
    #[validate(length(min = 1, message = "Kind must not be empty"))]
    pub kind: String,
    #[serde(rename = "requestId")]
    #[validate(length(min = 1, message = "Request ID must not be empty"))]
    pub request_id: String,
    pub api: Option<String>,
}
