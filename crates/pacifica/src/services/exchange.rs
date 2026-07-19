//! Pacifica REST trading: market-order placement on behalf of a user.
//!
//! Signing scheme (Pacifica Python SDK, `common/utils.py::sign_message`):
//!   1. Build a `header` dict: `{type, timestamp, expiry_window}`.
//!   2. Build a `payload` dict with the operation-specific fields.
//!   3. Wrap as `{**header, "data": payload}`.
//!   4. Recursively sort keys alphabetically.
//!   5. Serialize as compact JSON (`separators=(",",":") `) and UTF-8 encode.
//!   6. Sign the bytes with the agent's Ed25519 private key.
//!   7. Base58-encode the 64-byte signature for transport.
//!
//! The recursive sort + compact serialization comes "for free" with
//! `serde_json` because its internal `Map` is `BTreeMap` (no
//! `preserve_order` feature). `serde_json::to_string` produces a compact,
//! sorted-key string.
//!
//! The wire request body merges header fields, payload fields, account
//! (master pubkey), agent_wallet, and signature into one flat object —
//! NOT under a `data` key. The `data` nesting only exists in the signed
//! preimage.

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::PACIFICA_HTTP_URL;

const CREATE_MARKET_PATH: &str = "/orders/create_market";
const SIGNATURE_TYPE_CREATE_MARKET: &str = "create_market_order";
/// Pacifica SDK default; the signature is rejected this many ms after
/// `timestamp`. 5 seconds is generous for round-tripping to Turnkey + HL.
const DEFAULT_EXPIRY_WINDOW_MS: u64 = 5_000;

/// Ed25519 signer abstraction. Pacifica signs raw UTF-8 bytes, returning
/// a 64-byte signature (we base58-encode for the wire ourselves).
#[async_trait]
pub trait SignerBackend: Send + Sync {
    async fn sign_message(&self, message: &[u8]) -> Result<[u8; 64]>;
}

#[derive(Debug, Clone)]
pub struct MarketOrderParams {
    pub symbol: String,
    /// "bid" (buy) or "ask" (sell).
    pub side: Side,
    pub amount: String,
    pub slippage_percent: String,
    pub reduce_only: bool,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    fn as_str(self) -> &'static str {
        match self {
            Side::Bid => "bid",
            Side::Ask => "ask",
        }
    }
}

pub struct PacificaClient {
    base_url: String,
    http: reqwest::Client,
}

impl PacificaClient {
    pub fn new() -> Self {
        Self {
            base_url: PACIFICA_HTTP_URL.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Place a market order on behalf of `account` (master), signed by
    /// `agent_wallet`. Both must be base58-encoded Solana pubkeys.
    /// `signer` must hold the Ed25519 private key matching `agent_wallet`.
    pub async fn create_market_order(
        &self,
        account: &str,
        agent_wallet: &str,
        params: &MarketOrderParams,
        signer: &dyn SignerBackend,
    ) -> Result<Value> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time before UNIX epoch")?
            .as_millis() as u64;

        let header = SignHeader {
            r#type: SIGNATURE_TYPE_CREATE_MARKET,
            timestamp: timestamp_ms,
            expiry_window: DEFAULT_EXPIRY_WINDOW_MS,
        };

        // The signed payload uses *just* the operation fields. account /
        // agent_wallet / signature go on the outer envelope only.
        let payload = json!({
            "symbol": params.symbol,
            "side": params.side.as_str(),
            "amount": params.amount,
            "slippage_percent": params.slippage_percent,
            "reduce_only": params.reduce_only,
            "client_order_id": params.client_order_id,
        });

        let message_bytes = build_signed_message(&header, &payload)?;
        let signature_bytes = signer
            .sign_message(&message_bytes)
            .await
            .context("pacifica signer")?;
        let signature_b58 = bs58::encode(signature_bytes).into_string();

        // Wire body: flat, with header fields, payload fields, and the
        // signing-envelope fields merged together.
        let mut body = serde_json::Map::new();
        body.insert("account".into(), Value::String(account.to_string()));
        body.insert(
            "agent_wallet".into(),
            Value::String(agent_wallet.to_string()),
        );
        body.insert("signature".into(), Value::String(signature_b58));
        body.insert("timestamp".into(), Value::Number(timestamp_ms.into()));
        body.insert(
            "expiry_window".into(),
            Value::Number(DEFAULT_EXPIRY_WINDOW_MS.into()),
        );
        body.insert("symbol".into(), Value::String(params.symbol.clone()));
        body.insert(
            "side".into(),
            Value::String(params.side.as_str().to_string()),
        );
        body.insert("amount".into(), Value::String(params.amount.clone()));
        body.insert(
            "slippage_percent".into(),
            Value::String(params.slippage_percent.clone()),
        );
        body.insert("reduce_only".into(), Value::Bool(params.reduce_only));
        if let Some(cid) = &params.client_order_id {
            body.insert("client_order_id".into(), Value::String(cid.clone()));
        }

        let url = format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            CREATE_MARKET_PATH
        );

        let response = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .json(&Value::Object(body))
            .send()
            .await
            .context("Pacifica HTTP request failed")?;

        let status = response.status();
        let text = response
            .text()
            .await
            .context("reading Pacifica response body")?;

        if !status.is_success() {
            return Err(anyhow!("Pacifica HTTP {status}: {text}"));
        }

        let response_json: Value =
            serde_json::from_str(&text).context("decoding Pacifica response as JSON")?;

        validate_create_market_order_response(&response_json, params)?;

        Ok(response_json)
    }
}

/// Validates Pacifica's application-level acknowledgement after HTTP success.
///
/// A 2xx status only confirms that the transport succeeded. Pacifica documents
/// a successful market-order acknowledgement as code=200 with an exchange order
/// id and the requested symbol. Rejecting incomplete or mismatched payloads
/// prevents the automation layer from recording an order as accepted when the
/// venue rejected it at the business-logic layer.
fn validate_create_market_order_response(
    response: &Value,
    params: &MarketOrderParams,
) -> Result<()> {
    let code = response.get("code").and_then(Value::as_i64);
    if code != Some(200) {
        return Err(anyhow!(
            "Pacifica rejected market order with application code {:?}",
            code
        ));
    }

    let response_type = response.get("type").and_then(Value::as_str);
    if response_type != Some(SIGNATURE_TYPE_CREATE_MARKET) {
        return Err(anyhow!(
            "Pacifica returned unexpected response type {:?}",
            response_type
        ));
    }

    let data = response
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Pacifica success response is missing data"))?;

    if data.get("i").and_then(Value::as_u64).is_none() {
        return Err(anyhow!(
            "Pacifica success response is missing the exchange order id"
        ));
    }

    if data.get("s").and_then(Value::as_str) != Some(params.symbol.as_str()) {
        return Err(anyhow!(
            "Pacifica success response symbol does not match requested symbol"
        ));
    }

    if let Some(expected_cloid) = &params.client_order_id {
        if data.get("I").and_then(Value::as_str) != Some(expected_cloid.as_str()) {
            return Err(anyhow!(
                "Pacifica success response client order id does not match request"
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct SignHeader {
    r#type: &'static str,
    timestamp: u64,
    expiry_window: u64,
}

/// Build the canonical signed-message bytes per Pacifica's signing rule.
/// `serde_json::to_vec` produces compact JSON; map keys are alphabetically
/// sorted by default (without the `preserve_order` feature).
fn build_signed_message(header: &SignHeader, payload: &Value) -> Result<Vec<u8>> {
    let envelope = json!({
        "type": header.r#type,
        "timestamp": header.timestamp,
        "expiry_window": header.expiry_window,
        "data": payload,
    });
    serde_json::to_vec(&envelope).context("serializing pacifica signing envelope")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify serde_json's default ordering is alphabetical — this is the
    /// load-bearing assumption that lets us match Pacifica's
    /// `sort_json_keys` + compact-JSON signing. If this ever fails,
    /// signatures will silently start to differ from the Python SDK's.
    #[test]
    fn canonical_envelope_keys_are_sorted() {
        let header = SignHeader {
            r#type: "create_market_order",
            timestamp: 1_700_000_000_000,
            expiry_window: 5_000,
        };
        let payload = json!({
            "symbol": "BTC",
            "side": "bid",
            "amount": "0.1",
            "slippage_percent": "0.5",
            "reduce_only": false,
            "client_order_id": null,
        });
        let bytes = build_signed_message(&header, &payload).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        // Top-level keys: data, expiry_window, timestamp, type.
        // Find the position of each and assert ascending.
        let data = s.find("\"data\"").unwrap();
        let expiry = s.find("\"expiry_window\"").unwrap();
        let ts = s.find("\"timestamp\"").unwrap();
        let ty = s.find("\"type\"").unwrap();
        assert!(data < expiry, "data before expiry_window");
        assert!(expiry < ts, "expiry_window before timestamp");
        assert!(ts < ty, "timestamp before type");
        // Compact (no spaces).
        assert!(!s.contains(", "));
        assert!(!s.contains(": "));
    }
}
