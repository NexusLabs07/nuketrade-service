//! Turnkey raw-payload signing client.
//!
//! Single responsibility: given a pre-computed 32-byte digest, return a
//! 65-byte EVM signature produced by a specific Turnkey wallet. Used by
//! the automation worker to sign Hyperliquid EIP-712 action digests with
//! the agent's wallet.
//!
//! Authentication to Turnkey uses the existing P-256 API key (the same
//! pattern as the read-side calls in `services::auth`). Authorization to
//! actually use the target wallet is governed by Turnkey policies that
//! must already be in place — this client has no awareness of those.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::ecdsa::{Signature as P256Signature, SigningKey, signature::Signer};
use perp_core::config::Config;
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};

const TURNKEY_SIGNATURE_SCHEME: &str = "SIGNATURE_SCHEME_TK_API_P256";
const SIGN_RAW_PAYLOAD_PATH: &str = "public/v1/submit/sign_raw_payload";

#[derive(Clone)]
pub struct TurnkeyClient {
    http: Client,
    base_url: String,
    parent_org_id: String,
    api_public_key: String,
    api_private_key: String,
}

impl TurnkeyClient {
    pub fn from_config(config: &Config) -> Self {
        Self {
            http: Client::new(),
            base_url: config.turnkey_api_base_url.clone(),
            parent_org_id: config.turnkey_parent_org_id.clone(),
            api_public_key: config.turnkey_api_public_key.clone(),
            api_private_key: config.turnkey_api_private_key.clone(),
        }
    }

    /// Sign a 32-byte digest. The digest is sent as-is to Turnkey
    /// (HASH_FUNCTION_NO_OP) — caller is responsible for any pre-hashing
    /// such as EIP-712.
    ///
    /// `suborg_id` is the sub-organization that owns the signing wallet
    /// (for the agent: `config.agent_turnkey_suborg_id`).
    /// `sign_with` is what Turnkey calls the resource — typically the
    /// wallet account's EVM address (e.g. `0xabc…`).
    pub async fn sign_raw_payload(
        &self,
        suborg_id: &str,
        sign_with: &str,
        digest: &[u8; 32],
    ) -> Result<EvmSignature> {
        let result = self
            .sign_raw_bytes(suborg_id, sign_with, digest.as_slice())
            .await?;
        EvmSignature::from_rsv_hex(&result.r, &result.s, &result.v)
    }

    /// Sign arbitrary bytes with a Solana (Ed25519) wallet. Pacifica's
    /// signing scheme signs a UTF-8-encoded canonical JSON string, which
    /// can be longer than 32 bytes — this method doesn't constrain the
    /// length the way `sign_raw_payload` does.
    ///
    /// The 64-byte Ed25519 signature is reconstructed from Turnkey's
    /// `r` + `s` hex (each 32 bytes). `v` is always empty for Ed25519
    /// and is ignored.
    pub async fn sign_solana_payload(
        &self,
        suborg_id: &str,
        sign_with: &str,
        message: &[u8],
    ) -> Result<[u8; 64]> {
        let result = self.sign_raw_bytes(suborg_id, sign_with, message).await?;
        let r = decode_32_hex(&result.r, "r")?;
        let s = decode_32_hex(&result.s, "s")?;
        let mut sig = [0u8; 64];
        sig[..32].copy_from_slice(&r);
        sig[32..].copy_from_slice(&s);
        Ok(sig)
    }

    /// Shared Turnkey activity submission for both EVM and Solana paths.
    /// Returns the `r/s/v` result without imposing an interpretation.
    async fn sign_raw_bytes(
        &self,
        suborg_id: &str,
        sign_with: &str,
        bytes: &[u8],
    ) -> Result<SignRawPayloadResult> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time before UNIX epoch")?
            .as_millis()
            .to_string();
        let payload_hex = format!("0x{}", hex::encode(bytes));

        let body = serde_json::json!({
            "type": "ACTIVITY_TYPE_SIGN_RAW_PAYLOAD_V2",
            "timestampMs": timestamp_ms,
            "organizationId": suborg_id,
            "parameters": {
                "signWith": sign_with,
                "payload": payload_hex,
                "encoding": "PAYLOAD_ENCODING_HEXADECIMAL",
                "hashFunction": "HASH_FUNCTION_NO_OP"
            }
        });

        let resp: SignRawPayloadResponse = self.stamped_post(SIGN_RAW_PAYLOAD_PATH, &body).await?;

        if resp.activity.status != "ACTIVITY_STATUS_COMPLETED" {
            return Err(anyhow!(
                "Turnkey sign_raw_payload not completed: status={}",
                resp.activity.status
            ));
        }

        resp.activity
            .result
            .and_then(|r| r.sign_raw_payload_result)
            .ok_or_else(|| anyhow!("Turnkey response missing signRawPayloadResult"))
    }

    async fn stamped_post<TReq, TRes>(&self, path: &str, payload: &TReq) -> Result<TRes>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let body = serde_json::to_vec(payload).context("encoding Turnkey payload")?;
        let stamp = self.build_stamp(&body)?;
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Organization-Id", &self.parent_org_id)
            .header("X-Stamp", stamp)
            .body(body)
            .send()
            .await
            .context("Turnkey HTTP request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Turnkey request failed: status={status}, body={body}"
            ));
        }

        response
            .json::<TRes>()
            .await
            .context("decoding Turnkey response")
    }

    fn build_stamp(&self, body: &[u8]) -> Result<String> {
        let private_key_hex = self.api_private_key.trim().trim_start_matches("0x");
        let private_key_bytes =
            hex::decode(private_key_hex).context("invalid TURNKEY_API_PRIVATE_KEY hex")?;
        let key: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| anyhow!("TURNKEY_API_PRIVATE_KEY must decode to exactly 32 bytes"))?;

        let signing_key =
            SigningKey::from_bytes((&key).into()).context("invalid Turnkey signing key bytes")?;
        let signature: P256Signature = signing_key.sign(body);
        let der_signature = signature.to_der();

        let stamp_payload = serde_json::json!({
            "publicKey": self.api_public_key,
            "signature": hex::encode(der_signature.as_bytes()),
            "scheme": TURNKEY_SIGNATURE_SCHEME
        });

        let stamp_json = serde_json::to_vec(&stamp_payload).context("encoding stamp payload")?;
        Ok(URL_SAFE_NO_PAD.encode(stamp_json))
    }
}

/// 65-byte (r, s, v) EVM signature returned by SIGN_RAW_PAYLOAD_V2.
/// `v` is the raw recovery byte (0 or 1); EIP-191/-712 consumers (HL,
/// EVM ecrecover) expect `v + 27`.
#[derive(Debug, Clone, Copy)]
pub struct EvmSignature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
}

impl EvmSignature {
    fn from_rsv_hex(r: &str, s: &str, v: &str) -> Result<Self> {
        Ok(Self {
            r: decode_32_hex(r, "r")?,
            s: decode_32_hex(s, "s")?,
            v: decode_v_byte(v)?,
        })
    }

    /// `v + 27` for EIP-712 consumers (Hyperliquid expects 27 or 28).
    pub fn v_eip712(&self) -> u8 {
        self.v + 27
    }
}

fn decode_32_hex(s: &str, field: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(s.trim_start_matches("0x"))
        .with_context(|| format!("invalid hex for {field}: {s}"))?;
    if bytes.len() != 32 {
        return Err(anyhow!("{field} must be 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_v_byte(v: &str) -> Result<u8> {
    let bytes =
        hex::decode(v.trim_start_matches("0x")).with_context(|| format!("invalid v hex: {v}"))?;
    match bytes.as_slice() {
        [b] => Ok(*b),
        [] => Err(anyhow!("Turnkey returned empty v")),
        _ => Err(anyhow!("Turnkey returned v longer than 1 byte: {v}")),
    }
}

#[derive(Debug, serde::Deserialize)]
struct SignRawPayloadResponse {
    activity: TurnkeyActivity,
}

#[derive(Debug, serde::Deserialize)]
struct TurnkeyActivity {
    status: String,
    #[serde(default)]
    result: Option<TurnkeyActivityResult>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TurnkeyActivityResult {
    #[serde(default)]
    sign_raw_payload_result: Option<SignRawPayloadResult>,
}

#[derive(Debug, serde::Deserialize)]
struct SignRawPayloadResult {
    r: String,
    s: String,
    v: String,
}
