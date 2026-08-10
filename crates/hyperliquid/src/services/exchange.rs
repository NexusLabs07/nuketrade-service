//! Hyperliquid `/exchange` order placement.
//!
//! HL's signing scheme isn't standard JSON-EIP-712. The 4-step pipeline:
//!
//!   1. Build the action as a struct.
//!   2. msgpack-encode the action (HL hashes the msgpack bytes, not JSON;
//!      struct field declaration order is preserved through serialization
//!      and matters byte-for-byte).
//!   3. Action hash = keccak256( msgpack || nonce_u64_be || vault_byte
//!      || optional_vault_addr ). vault_byte is 0x00 with no vault, 0x01
//!      followed by 20 bytes otherwise.
//!   4. EIP-712 wrap: the action hash is the `connectionId` field of an
//!      `Agent { source, connectionId }` typed-data message. Domain has
//!      chainId 1337 regardless of L1 — that's an HL convention.
//!
//! Signing is abstracted behind `SignerBackend` so the worker plugs in
//! Turnkey while tests can use a local key.

use alloy::primitives::{Address, B256, keccak256};
use alloy::sol;
use alloy::sol_types::{Eip712Domain, SolStruct, eip712_domain};
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

const HL_MAINNET_URL: &str = "https://api.hyperliquid.xyz/exchange";
const HL_TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz/exchange";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HlEnv {
    Mainnet,
    Testnet,
}

impl HlEnv {
    fn exchange_url(self) -> &'static str {
        match self {
            HlEnv::Mainnet => HL_MAINNET_URL,
            HlEnv::Testnet => HL_TESTNET_URL,
        }
    }

    fn agent_source(self) -> &'static str {
        // HL uses a one-letter discriminator inside the EIP-712 Agent
        // message: "a" mainnet, "b" testnet.
        match self {
            HlEnv::Mainnet => "a",
            HlEnv::Testnet => "b",
        }
    }
}

// HL's EIP-712 typed-data message. The action hash from step 3 goes into
// `connectionId`; `source` distinguishes mainnet vs testnet so the same
// signature can't be replayed across networks.
sol! {
    struct Agent {
        string source;
        bytes32 connectionId;
    }
}

fn hl_domain() -> Eip712Domain {
    eip712_domain! {
        name: "Exchange",
        version: "1",
        chain_id: 1337,
        verifying_contract: Address::ZERO,
    }
}

// ────────── Action types ──────────
//
// Field order in these structs is the WIRE order. Do not reorder.

/// A single order leg inside the `order` action.
/// Short field names (`a`, `b`, `p`, …) are HL's wire schema, not stylistic.
#[derive(Debug, Clone, Serialize)]
pub struct OrderRequest {
    /// Asset index (position in HL's unfiltered `universe` array — NOT the
    /// filtered `HL_MARKETS` Vec; see worker for the correct lookup).
    pub a: u32,
    /// `true` = buy / long, `false` = sell / short.
    pub b: bool,
    /// Price as decimal string, already rounded to HL's tick size.
    pub p: String,
    /// Size as decimal string, already rounded to the asset's `szDecimals`.
    pub s: String,
    /// Reduce-only flag.
    pub r: bool,
    /// Order type (`limit` with TIF for now; trigger orders not modeled).
    pub t: OrderTypeWire,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderTypeWire {
    pub limit: LimitWire,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitWire {
    /// `"Gtc"` (default), `"Ioc"` (market-ish), or `"Alo"` (post-only).
    pub tif: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExchangeAction {
    Order {
        orders: Vec<OrderRequest>,
        /// `"na"` for vanilla orders; HL also supports `"normalTpsl"`,
        /// `"positionTpsl"` for grouped TP/SL legs — out of scope here.
        grouping: String,
    },
    /// Cancel by (asset, order id) pairs.
    Cancel { cancels: Vec<CancelRequest> },
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelRequest {
    pub a: u32,
    pub o: u64,
}

// ────────── Signer abstraction ──────────

/// What `HlClient` needs from a signer: turn a 32-byte EIP-712 digest into
/// a 65-byte signature. The 65th byte is `v` (27 or 28) — EIP-712 form.
#[async_trait]
pub trait SignerBackend: Send + Sync {
    async fn sign_digest(&self, digest: &[u8; 32]) -> Result<HlSignature>;
}

#[derive(Debug, Clone, Copy)]
pub struct HlSignature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    /// 27 or 28 (EIP-712 form, not 0/1).
    pub v: u8,
}

// ────────── Client ──────────

pub struct HlClient {
    env: HlEnv,
    http: reqwest::Client,
}

impl HlClient {
    pub fn new(env: HlEnv) -> Self {
        Self {
            env,
            http: reqwest::Client::new(),
        }
    }

    /// End-to-end: encode → hash → digest → sign → POST → parse.
    pub async fn submit_action(
        &self,
        action: &ExchangeAction,
        signer: &dyn SignerBackend,
    ) -> Result<serde_json::Value> {
        let nonce_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time before UNIX epoch")?
            .as_millis() as u64;

        let action_bytes = msgpack_encode(action)?;
        let action_hash = compute_action_hash(&action_bytes, nonce_ms, None);
        let digest = self.eip712_digest(action_hash);
        let signature = signer.sign_digest(&digest).await?;

        let body = serde_json::json!({
            "action": action,
            "nonce": nonce_ms,
            "signature": {
                "r": format!("0x{}", hex::encode(signature.r)),
                "s": format!("0x{}", hex::encode(signature.s)),
                "v": signature.v,
            },
        });

        let response = self
            .http
            .post(self.env.exchange_url())
            .json(&body)
            .send()
            .await
            .context("HL HTTP request failed")?;

        let status = response.status();
        let text = response.text().await.context("reading HL response body")?;

        if !status.is_success() {
            return Err(anyhow!("HL HTTP {status}: {text}"));
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).context("decoding HL response as JSON")?;

        if json.get("status").and_then(|s| s.as_str()) != Some("ok") {
            return Err(anyhow!("HL rejected action: {text}"));
        }

        Ok(json)
    }

    fn eip712_digest(&self, action_hash: [u8; 32]) -> [u8; 32] {
        let agent = Agent {
            source: self.env.agent_source().to_string(),
            connectionId: B256::from(action_hash),
        };
        agent.eip712_signing_hash(&hl_domain()).0
    }
}

// ────────── Encoding helpers ──────────

fn msgpack_encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    // `with_struct_map()` makes structs encode as named-key maps (matching
    // HL's TypeScript SDK), not positional tuples.
    let mut buf = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut buf).with_struct_map();
    value
        .serialize(&mut serializer)
        .context("msgpack encoding action")?;
    Ok(buf)
}

fn compute_action_hash(action_bytes: &[u8], nonce_ms: u64, vault: Option<Address>) -> [u8; 32] {
    let mut buf = Vec::with_capacity(action_bytes.len() + 8 + 21);
    buf.extend_from_slice(action_bytes);
    buf.extend_from_slice(&nonce_ms.to_be_bytes());
    match vault {
        None => buf.push(0x00),
        Some(addr) => {
            buf.push(0x01);
            buf.extend_from_slice(addr.as_slice());
        }
    }
    keccak256(&buf).0
}

// ────────── Tests ──────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity: identical (action, nonce) produces an identical action hash.
    /// Catches regressions in field ordering / msgpack encoding flags.
    #[test]
    fn action_hash_is_deterministic() {
        let action = ExchangeAction::Order {
            orders: vec![OrderRequest {
                a: 0,
                b: true,
                p: "50000.0".to_string(),
                s: "0.01".to_string(),
                r: false,
                t: OrderTypeWire {
                    limit: LimitWire {
                        tif: "Gtc".to_string(),
                    },
                },
            }],
            grouping: "na".to_string(),
        };
        let bytes = msgpack_encode(&action).unwrap();
        let h1 = compute_action_hash(&bytes, 1_700_000_000_000, None);
        let h2 = compute_action_hash(&bytes, 1_700_000_000_000, None);
        assert_eq!(h1, h2);
        // Different nonce → different hash.
        let h3 = compute_action_hash(&bytes, 1_700_000_000_001, None);
        assert_ne!(h1, h3);
    }
}
