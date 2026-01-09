/**
 * Credit to https://github.com/nktkas/hyperliquid/blob/main/src/signing.ts
 *
 * This module contains functions for generating Hyperliquid transaction signatures
 * and interfaces to various wallet implementations.
 */

use ethers::core::types::{Signature as EthSignature, U256};
use ethers::signers::{LocalWallet, Signer};
use ethers::utils::keccak256;
use hex::{decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha3::{Digest, Keccak256};

pub type Hex = String;

#[derive(Debug)]
pub enum SigningError {
    UnsupportedWallet,
    NoAccounts,
    SigningFailed(String),
    InvalidSignature,
    SerializationError(String),
    HexDecodeError(String),
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::UnsupportedWallet => write!(f, "Unsupported wallet for signing typed data"),
            SigningError::NoAccounts => write!(f, "No Ethereum accounts available"),
            SigningError::SigningFailed(msg) => write!(f, "Failed to sign: {}", msg),
            SigningError::InvalidSignature => write!(f, "Invalid signature format"),
            SigningError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            SigningError::HexDecodeError(msg) => write!(f, "Hex decode error: {}", msg),
        }
    }
}

impl std::error::Error for SigningError {}

/// Signature components r, s, and v
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub r: Hex,
    pub s: Hex,
    pub v: u8,
}

/// Cancel action structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub cancels: Vec<CancelItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelItem {
    pub a: u32,
    pub o: u64,
}

/// Transfer action for mainnet
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub hyperliquid_chain: String,
    pub signature_chain_id: String,
    pub amount: String,
    pub to_perp: bool,
    pub nonce: u64,
}

/// Transfer action for testnet
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferActionTestnet {
    #[serde(rename = "type")]
    pub action_type: String,
    pub hyperliquid_chain: String,
    pub signature_chain_id: String,
    pub amount: String,
    pub to_perp: bool,
    pub nonce: u64,
}

/// Agent message for Exchange domain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub source: String,
    pub connection_id: Hex,
}

/// Normalize integers for MessagePack compatibility
fn normalize_integers_for_msgpack(value: &JsonValue) -> JsonValue {
    const THIRTY_ONE_BITS: i64 = 2147483648;
    const THIRTY_TWO_BITS: i64 = 4294967296;

    match value {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= THIRTY_TWO_BITS || i < -THIRTY_ONE_BITS {
                    // Convert to string representation for large integers
                    return JsonValue::String(i.to_string());
                }
            }
            value.clone()
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(normalize_integers_for_msgpack).collect())
        }
        JsonValue::Object(obj) => JsonValue::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), normalize_integers_for_msgpack(v)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/**
 * Create a hash of the L1 action.
 *
 * Note: Hash generation depends on the order of the action keys.
 *
 * # Arguments
 * * `action` - The action to be hashed (as JSON value)
 * * `nonce` - Unique request identifier (recommended current timestamp in ms)
 * * `vault_address` - Optional vault address used in the action
 *
 * # Returns
 * The hash of the action as a hex string
 */
pub fn create_l1_action_hash(
    action: &JsonValue,
    nonce: u64,
    vault_address: Option<&str>,
) -> Result<Hex, SigningError> {
    let normalized_action = normalize_integers_for_msgpack(action);

    let msgpack_bytes = rmp_serde::to_vec(&normalized_action)
        .map_err(|e| SigningError::SerializationError(e.to_string()))?;

    let additional_bytes_length = if vault_address.is_some() { 29 } else { 9 };
    let mut data = vec![0u8; msgpack_bytes.len() + additional_bytes_length];
    data[..msgpack_bytes.len()].copy_from_slice(&msgpack_bytes);

    // Add nonce as big-endian u64
    let nonce_bytes = nonce.to_be_bytes();
    data[msgpack_bytes.len()..msgpack_bytes.len() + 8].copy_from_slice(&nonce_bytes);

    if let Some(vault_addr) = vault_address {
        data[msgpack_bytes.len() + 8] = 1;
        let vault_bytes = decode(vault_addr.trim_start_matches("0x"))
            .map_err(|e| SigningError::HexDecodeError(e.to_string()))?;
        data[msgpack_bytes.len() + 9..msgpack_bytes.len() + 29].copy_from_slice(&vault_bytes);
    } else {
        data[msgpack_bytes.len() + 8] = 0;
    }

    let mut hasher = Keccak256::new();
    hasher.update(&data);
    let hash = hasher.finalize();

    Ok(format!("0x{}", encode(hash)))
}

/// Helper to encode EIP-712 typed data hash
fn encode_eip712_message(
    domain_separator: [u8; 32],
    struct_hash: [u8; 32],
) -> [u8; 32] {
    let mut message = Vec::with_capacity(2 + 32 + 32);
    message.push(0x19);
    message.push(0x01);
    message.extend_from_slice(&domain_separator);
    message.extend_from_slice(&struct_hash);
    keccak256(&message)
}

/// Create domain separator for EIP-712
fn create_domain_separator(
    name: &str,
    version: &str,
    chain_id: u64,
    verifying_contract: &str,
) -> [u8; 32] {
    let domain_type_hash = keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
    let name_hash = keccak256(name.as_bytes());
    let version_hash = keccak256(version.as_bytes());

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&domain_type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    // Chain ID as uint256 (32 bytes)
    let mut chain_id_bytes = [0u8; 32];
    U256::from(chain_id).to_big_endian(&mut chain_id_bytes);
    encoded.extend_from_slice(&chain_id_bytes);
    // Verifying contract address (20 bytes, left-padded to 32)
    let contract_bytes = decode(verifying_contract.trim_start_matches("0x")).unwrap();
    let mut padded_contract = [0u8; 32];
    padded_contract[12..].copy_from_slice(&contract_bytes);
    encoded.extend_from_slice(&padded_contract);

    keccak256(&encoded)
}

/// Create struct hash for Agent message
fn create_agent_struct_hash(source: &str, connection_id: &str) -> [u8; 32] {
    let type_hash = keccak256("Agent(string source,bytes32 connectionId)");
    let source_hash = keccak256(source.as_bytes());
    let connection_id_bytes = decode(connection_id.trim_start_matches("0x")).unwrap();

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&source_hash);
    encoded.extend_from_slice(&connection_id_bytes);

    keccak256(&encoded)
}

/**
 * Sign an L1 action.
 *
 * Note: Signature generation depends on the order of the action keys.
 */
pub async fn sign_l1_action(
    wallet: &LocalWallet,
    action: &JsonValue,
    nonce: u64,
    is_testnet: bool,
    vault_address: Option<&str>,
) -> Result<Signature, SigningError> {
    let action_hash = create_l1_action_hash(action, nonce, vault_address)?;

    // Create domain separator
    let domain_separator = create_domain_separator(
        "Exchange",
        "1",
        1337,
        "0x0000000000000000000000000000000000000000",
    );

    // Create struct hash
    let source = if is_testnet { "b" } else { "a" };
    let struct_hash = create_agent_struct_hash(source, &action_hash);

    // Create EIP-712 message hash
    let message_hash = encode_eip712_message(domain_separator, struct_hash);

    // Sign the message
    let signature = wallet
        .sign_message(&message_hash)
        .await
        .map_err(|e| SigningError::SigningFailed(e.to_string()))?;

    split_signature(&signature)
}

/**
 * Sign a cancel action (same as signL1Action but doesn't convert bigints).
 */
pub async fn sign_cancel_action(
    wallet: &LocalWallet,
    action: &JsonValue,
    nonce: u64,
    is_testnet: bool,
    vault_address: Option<&str>,
) -> Result<Signature, SigningError> {
    // Create hash without BigInt conversion
    let msgpack_bytes = rmp_serde::to_vec(action)
        .map_err(|e| SigningError::SerializationError(e.to_string()))?;

    let additional_bytes_length = if vault_address.is_some() { 29 } else { 9 };
    let mut data = vec![0u8; msgpack_bytes.len() + additional_bytes_length];
    data[..msgpack_bytes.len()].copy_from_slice(&msgpack_bytes);

    let nonce_bytes = nonce.to_be_bytes();
    data[msgpack_bytes.len()..msgpack_bytes.len() + 8].copy_from_slice(&nonce_bytes);

    if let Some(vault_addr) = vault_address {
        data[msgpack_bytes.len() + 8] = 1;
        let vault_bytes = decode(vault_addr.trim_start_matches("0x"))
            .map_err(|e| SigningError::HexDecodeError(e.to_string()))?;
        data[msgpack_bytes.len() + 9..msgpack_bytes.len() + 29].copy_from_slice(&vault_bytes);
    } else {
        data[msgpack_bytes.len() + 8] = 0;
    }

    let mut hasher = Keccak256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    let action_hash = format!("0x{}", encode(hash));

    // Create domain separator
    let domain_separator = create_domain_separator(
        "Exchange",
        "1",
        1337,
        "0x0000000000000000000000000000000000000000",
    );

    // Create struct hash
    let source = if is_testnet { "b" } else { "a" };
    let struct_hash = create_agent_struct_hash(source, &action_hash);

    // Create EIP-712 message hash
    let message_hash = encode_eip712_message(domain_separator, struct_hash);

    // Sign the message
    let signature = wallet
        .sign_message(&message_hash)
        .await
        .map_err(|e| SigningError::SigningFailed(e.to_string()))?;

    split_signature(&signature)
}

/// Create struct hash for UsdClassTransfer message
fn create_usd_class_transfer_struct_hash(
    hyperliquid_chain: &str,
    amount: &str,
    to_perp: bool,
    nonce: u64,
) -> [u8; 32] {
    let type_hash = keccak256("HyperliquidTransaction:UsdClassTransfer(string hyperliquidChain,string amount,bool toPerp,uint64 nonce)");
    let chain_hash = keccak256(hyperliquid_chain.as_bytes());
    let amount_hash = keccak256(amount.as_bytes());

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&chain_hash);
    encoded.extend_from_slice(&amount_hash);
    // toPerp as uint256 (bool is encoded as uint256)
    let mut to_perp_bytes = [0u8; 32];
    if to_perp {
        to_perp_bytes[31] = 1;
    }
    encoded.extend_from_slice(&to_perp_bytes);
    // nonce as uint64 (still 32 bytes in EIP-712)
    let mut nonce_bytes = [0u8; 32];
    U256::from(nonce).to_big_endian(&mut nonce_bytes);
    encoded.extend_from_slice(&nonce_bytes);

    keccak256(&encoded)
}

/**
 * Sign a transfer action for mainnet.
 */
pub async fn sign_transfer_action(
    wallet: &LocalWallet,
    hyperliquid_chain: &str,
    amount: &str,
    to_perp: bool,
    nonce: u64,
) -> Result<Signature, SigningError> {
    // Create domain separator
    let domain_separator = create_domain_separator(
        "HyperliquidSignTransaction",
        "1",
        42161, // Arbitrum Mainnet
        "0x0000000000000000000000000000000000000000",
    );

    // Create struct hash
    let struct_hash = create_usd_class_transfer_struct_hash(hyperliquid_chain, amount, to_perp, nonce);

    // Create EIP-712 message hash
    let message_hash = encode_eip712_message(domain_separator, struct_hash);

    // Sign the message
    let signature = wallet
        .sign_message(&message_hash)
        .await
        .map_err(|e| SigningError::SigningFailed(e.to_string()))?;

    split_signature(&signature)
}

/**
 * Sign a transfer action for testnet.
 */
pub async fn sign_transfer_action_testnet(
    wallet: &LocalWallet,
    hyperliquid_chain: &str,
    amount: &str,
    to_perp: bool,
    nonce: u64,
) -> Result<Signature, SigningError> {
    // Create domain separator
    let domain_separator = create_domain_separator(
        "HyperliquidSignTransaction",
        "1",
        421614, // Arbitrum Testnet
        "0x0000000000000000000000000000000000000000",
    );

    // Create struct hash
    let struct_hash = create_usd_class_transfer_struct_hash(hyperliquid_chain, amount, to_perp, nonce);

    // Create EIP-712 message hash
    let message_hash = encode_eip712_message(domain_separator, struct_hash);

    // Sign the message
    let signature = wallet
        .sign_message(&message_hash)
        .await
        .map_err(|e| SigningError::SigningFailed(e.to_string()))?;

    split_signature(&signature)
}

/**
 * Split a signature into its components r, s, and v.
 */
pub fn split_signature(signature: &EthSignature) -> Result<Signature, SigningError> {
    let sig_bytes = signature.to_vec();

    if sig_bytes.len() != 65 {
        return Err(SigningError::InvalidSignature);
    }

    let r = format!("0x{}", encode(&sig_bytes[0..32]));
    let s = format!("0x{}", encode(&sig_bytes[32..64]));
    let v = sig_bytes[64];

    Ok(Signature { r, s, v })
}

// Typed data creation functions (returns JSON for external use)

/// Create typed data for Exchange actions on testnet
pub fn create_testnet_exchange_typed_data(
    action: &JsonValue,
    nonce: u64,
    vault_address: Option<&str>,
) -> Result<serde_json::Value, SigningError> {
    Ok(serde_json::json!({
        "domain": {
            "name": "Exchange",
            "version": "1",
            "chainId": 1337,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "types": {
            "Agent": [
                { "name": "source", "type": "string" },
                { "name": "connectionId", "type": "bytes32" }
            ]
        },
        "primaryType": "Agent",
        "message": {
            "source": "b",
            "connectionId": create_l1_action_hash(action, nonce, vault_address)?
        }
    }))
}

/// Create typed data for Exchange actions on mainnet
pub fn create_mainnet_exchange_typed_data(
    action: &JsonValue,
    nonce: u64,
    vault_address: Option<&str>,
) -> Result<serde_json::Value, SigningError> {
    Ok(serde_json::json!({
        "domain": {
            "name": "Exchange",
            "version": "1",
            "chainId": 1337,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "types": {
            "Agent": [
                { "name": "source", "type": "string" },
                { "name": "connectionId", "type": "bytes32" }
            ]
        },
        "primaryType": "Agent",
        "message": {
            "source": "a",
            "connectionId": create_l1_action_hash(action, nonce, vault_address)?
        }
    }))
}

/// Create typed data for USD transfers on testnet
pub fn create_testnet_usd_transfer_typed_data(
    amount: &str,
    to_perp: bool,
    nonce: u64,
) -> serde_json::Value {
    serde_json::json!({
        "domain": {
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "chainId": 421614,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "types": {
            "HyperliquidTransaction:UsdClassTransfer": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "amount", "type": "string" },
                { "name": "toPerp", "type": "bool" },
                { "name": "nonce", "type": "uint64" }
            ]
        },
        "primaryType": "HyperliquidTransaction:UsdClassTransfer",
        "message": {
            "hyperliquidChain": "Testnet",
            "amount": amount,
            "toPerp": to_perp,
            "nonce": nonce
        }
    })
}

/// Create typed data for USD transfers on mainnet
pub fn create_mainnet_usd_transfer_typed_data(
    amount: &str,
    to_perp: bool,
    nonce: u64,
) -> serde_json::Value {
    serde_json::json!({
        "domain": {
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "chainId": 42161,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "types": {
            "HyperliquidTransaction:UsdClassTransfer": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "amount", "type": "string" },
                { "name": "toPerp", "type": "bool" },
                { "name": "nonce", "type": "uint64" }
            ]
        },
        "primaryType": "HyperliquidTransaction:UsdClassTransfer",
        "message": {
            "hyperliquidChain": "Mainnet",
            "amount": amount,
            "toPerp": to_perp,
            "nonce": nonce
        }
    })
}

/// Create typed data for USDC core transfers on mainnet
pub fn create_mainnet_usdc_core_transfer_typed_data(
    amount: &str,
    destination: &str,
    time: u64,
) -> serde_json::Value {
    serde_json::json!({
        "types": {
            "HyperliquidTransaction:UsdSend": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "destination", "type": "string" },
                { "name": "amount", "type": "string" },
                { "name": "time", "type": "uint64" }
            ]
        },
        "primaryType": "HyperliquidTransaction:UsdSend",
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

/// Create typed data for spot transfers on mainnet
pub fn create_mainnet_spot_transfer_typed_data(
    amount: &str,
    ticker: &str,
    ticker_address: &str,
    destination: &str,
    time: u64,
) -> serde_json::Value {
    serde_json::json!({
        "types": {
            "HyperliquidTransaction:SpotSend": [
                { "name": "hyperliquidChain", "type": "string" },
                { "name": "destination", "type": "string" },
                { "name": "token", "type": "string" },
                { "name": "amount", "type": "string" },
                { "name": "time", "type": "uint64" }
            ]
        },
        "primaryType": "HyperliquidTransaction:SpotSend",
        "domain": {
            "name": "HyperliquidSignTransaction",
            "version": "1",
            "chainId": 42161,
            "verifyingContract": "0x0000000000000000000000000000000000000000"
        },
        "message": {
            "hyperliquidChain": "Mainnet",
            "destination": destination,
            "token": format!("{}:{}", ticker, ticker_address),
            "amount": amount,
            "time": time
        }
    })
}

/// Create typed data for withdrawals
pub fn create_withdraw_typed_data(
    destination: &str,
    amount: &str,
    time: u64,
) -> serde_json::Value {
    serde_json::json!({
        "types": {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_l1_action_hash() {
        let action = serde_json::json!({
            "type": "cancel",
            "cancels": [{"a": 0, "o": 12345}]
        });
        let nonce = 1234567890;

        let result = create_l1_action_hash(&action, nonce, None);
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert!(hash.starts_with("0x"));
        assert_eq!(hash.len(), 66); // 0x + 64 hex chars
    }
}
