use std::{
    collections::HashMap,
    fmt,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy::primitives::{Address, Signature as EvmSignature};
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use p256::ecdsa::{Signature as P256signature, SigningKey, signature::Signer};
use perp_core::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{error::AppError, features::auth::types::AuthClaims};

const DEFAULT_PERMIT_BINDING_TTL_SECS: u64 = 15 * 60;
const ALLOWED_CLOCK_SKEW_SECS: u64 = 60;

const TURNKEY_SIGNATURE_SCHEME: &str = "SIGNATURE_SCHEME_TK_API_P256";
const ADDRESS_FORMAT_ETHEREUM: &str = "ADDRESS_FORMAT_ETHEREUM";
const ADDRESS_FORMAT_SOLANA: &str = "ADDRESS_FORMAT_SOLANA";

#[derive(Clone)]
pub struct AuthService {
    http: Client,
    turnkey_base_url: String,
    turnkey_parent_org_id: String,
    turnkey_api_public_key: String,
    turnkey_api_private_key: String,
    jwt_secret: String,
    jwt_ttl_secs: u64,
    challenge_ttl_secs: u64,
    challenges: Arc<RwLock<HashMap<String, StoredChallenge>>>,
    permit_bindings: Arc<RwLock<HashMap<String, StoredPermitBinding>>>,
}

impl fmt::Debug for AuthService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService")
            .field("turnkey_base_url", &self.turnkey_base_url)
            .field("turnkey_parent_org_id", &self.turnkey_parent_org_id)
            .field("jwt_ttl_secs", &self.jwt_ttl_secs)
            .field("challenge_ttl_secs", &self.challenge_ttl_secs)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct StoredChallenge {
    suborg_id: String,
    nonce: String,
    message: String,
    issued_at_unix: u64,
    expires_at_unix: u64,
}

#[derive(Debug, Clone)]
struct StoredPermitBinding {
    evm_address: Address,
    expires_at_unix: u64,
}

#[derive(Debug, Clone)]
struct ParsedChallengeMessage {
    suborg_id: String,
    nonce: String,
    issued_at_unix: u64,
    expires_at_unix: u64,
}

#[derive(Debug, Clone)]
struct TurnkeyAddresses {
    evm_addresses: Vec<Address>,
    solana_addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthLoginResult {
    pub token: String,
    pub evm_address: String,
    pub solana_address: String,
    pub expires_at_unix: u64,
}

impl AuthService {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to build reqwest client for AuthService")?;

        Ok(Self {
            http,
            turnkey_base_url: config.turnkey_api_base_url.clone(),
            turnkey_parent_org_id: config.turnkey_parent_org_id.clone(),
            turnkey_api_public_key: config.turnkey_api_public_key.clone(),
            turnkey_api_private_key: config.turnkey_api_private_key.clone(),
            jwt_secret: config.auth_jwt_secret.clone(),
            jwt_ttl_secs: config.auth_jwt_ttl_days.saturating_mul(24 * 60 * 60),
            challenge_ttl_secs: config.auth_challenge_ttl_secs,
            challenges: Arc::new(RwLock::new(HashMap::new())),
            permit_bindings: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn bind_permit_request_ids(
        &self,
        evm_address: &str,
        request_ids: &[String],
    ) -> Result<(), AppError> {
        if request_ids.is_empty() {
            return Ok(());
        }

        let evm_address = parse_evm_address(evm_address, "evm_address")?;
        let now = now_unix()?;
        let expires_at_unix = now.saturating_add(DEFAULT_PERMIT_BINDING_TTL_SECS);

        let mut lock = self.permit_bindings.write().await;
        lock.retain(|_, value| value.expires_at_unix > now);

        for request_id in request_ids {
            if request_id.is_empty() {
                continue;
            }

            lock.insert(
                request_id.clone(),
                StoredPermitBinding {
                    evm_address,
                    expires_at_unix,
                },
            );
        }

        Ok(())
    }

    pub async fn verify_permit_request_owner(
        &self,
        request_id: &str,
        evm_address: &str,
    ) -> Result<(), AppError> {
        let now = now_unix()?;
        let expected_evm = parse_evm_address(evm_address, "evm_address")?;

        let mut lock = self.permit_bindings.write().await;
        lock.retain(|_, value| value.expires_at_unix > now);

        let bound = lock
            .get(request_id)
            .ok_or_else(|| {
                AppError::unauthorised(
                    "unknown permit request_id; call /bridge/quote before /bridge/execute/permits",
                )
            })?
            .clone();

        if bound.evm_address != expected_evm {
            return Err(AppError::unauthorised(
                "permit request_id is not owned by authenticated EVM address",
            ));
        }

        Ok(())
    }

    pub async fn create_challenge(
        &self,
        suborg_id: String,
    ) -> Result<(String, String, u64), AppError> {
        if suborg_id.is_empty() {
            return Err(AppError::parse("suborgId", "suborgId must not be empty"));
        }

        let now = now_unix()?;
        let expires_at_unix = now.saturating_add(self.challenge_ttl_secs);
        let nonce = Uuid::new_v4().to_string();

        let message = format!(
            "nuketrade-login:v1|suborg:{}|nonce:{}|issued_at:{}|expires_at:{}",
            suborg_id, nonce, now, expires_at_unix
        );

        let mut lock = self.challenges.write().await;
        lock.retain(|_, value| value.expires_at_unix > now);

        lock.insert(
            nonce.clone(),
            StoredChallenge {
                suborg_id,
                nonce: nonce.clone(),
                message: message.clone(),
                issued_at_unix: now,
                expires_at_unix,
            },
        );

        Ok((message, nonce, expires_at_unix))
    }

    pub async fn login(
        &self,
        suborg_id: String,
        message: String,
        signature: String,
    ) -> Result<AuthLoginResult, AppError> {
        let parsed = parse_challenge_message(&message)?;

        if parsed.suborg_id != suborg_id {
            return Err(AppError::unauthorised(
                "suborgId mismatch in challenge message",
            ));
        }

        self.consume_challenge(&parsed, &message).await?;

        let recovered = recover_evm_address(&message, &signature)?;
        let turnkey_addresses = self.fetch_turnkey_addresses(&suborg_id).await?;

        if !turnkey_addresses.evm_addresses.contains(&recovered) {
            return Err(AppError::unauthorised(
                "signature address does not match any Turnkey EVM address",
            ));
        }

        let solana_address = turnkey_addresses
            .solana_addresses
            .first()
            .cloned()
            .ok_or_else(|| AppError::not_found("No Solana address found in Turnkey wallet"))?;

        let evm_address = recovered.to_string();

        let (token, expiry) =
            self.issue_jwt(suborg_id, evm_address.clone(), solana_address.clone())?;

        Ok(AuthLoginResult {
            token,
            evm_address,
            solana_address,
            expires_at_unix: expiry,
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<AuthClaims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 30;

        decode::<AuthClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map(|t| t.claims)
        .map_err(|_| AppError::unauthorised("invalid or expired bearer token"))
    }

    async fn consume_challenge(
        &self,
        parsed: &ParsedChallengeMessage,
        message: &str,
    ) -> Result<(), AppError> {
        let now = now_unix()?;
        let mut lock = self.challenges.write().await;
        lock.retain(|_, value| value.expires_at_unix > now);

        let stored = lock
            .remove(&parsed.nonce)
            .ok_or_else(|| AppError::unauthorised("challenge missing, expired, or already used"))?;

        if stored.suborg_id != parsed.suborg_id
            || stored.nonce != parsed.nonce
            || stored.message != message
            || stored.issued_at_unix != parsed.issued_at_unix
            || stored.expires_at_unix != parsed.expires_at_unix
        {
            return Err(AppError::unauthorised("challenge payload mismatch"));
        }

        if parsed.issued_at_unix > now.saturating_add(ALLOWED_CLOCK_SKEW_SECS) {
            return Err(AppError::unauthorised(
                "challenge issued_at is in the future",
            ));
        }

        if parsed.expires_at_unix <= now {
            return Err(AppError::unauthorised("challenge expired"));
        }

        Ok(())
    }

    fn issue_jwt(
        &self,
        suborg_id: String,
        evm_address: String,
        solana_address: String,
    ) -> Result<(String, u64), AppError> {
        let now = now_unix()?;
        let expiry = now.saturating_add(self.jwt_ttl_secs);

        let claims = AuthClaims {
            suborg_id,
            evm_address,
            solana_address,
            iat: now,
            exp: expiry,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::internal(format!("failed to issue jwt: {e}")))?;

        Ok((token, expiry))
    }

    async fn fetch_turnkey_addresses(&self, suborg_id: &str) -> Result<TurnkeyAddresses, AppError> {
        #[derive(Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ListWalletsRequest<'a> {
            organization_id: &'a str,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ListWalletsResponse {
            #[serde(default)]
            wallets: Vec<TurnkeyWallet>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TurnkeyWallet {
            wallet_id: String,
        }

        #[derive(Debug, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ListWalletAccountsRequest<'a> {
            organization_id: &'a str,
            wallet_id: &'a str,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ListWalletAccountsResponse {
            #[serde(default)]
            accounts: Vec<TurnkeyWalletAccount>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TurnkeyWalletAccount {
            address: String,
            address_format: String,
        }

        let wallets: ListWalletsResponse = self
            .stamped_post(
                "/public/v1/query/list_wallets",
                &ListWalletsRequest {
                    organization_id: suborg_id,
                },
            )
            .await?;

        if wallets.wallets.is_empty() {
            return Err(AppError::not_found("No Turnkey wallet found for suborg"));
        }

        let mut evm_addresses: Vec<Address> = Vec::new();
        let mut solana_addresses: Vec<String> = Vec::new();

        for wallet in wallets.wallets {
            let accounts: ListWalletAccountsResponse = self
                .stamped_post(
                    "/public/v1/query/list_wallet_accounts",
                    &ListWalletAccountsRequest {
                        organization_id: suborg_id,
                        wallet_id: &wallet.wallet_id,
                    },
                )
                .await?;

            for account in accounts.accounts {
                match account.address_format.as_str() {
                    ADDRESS_FORMAT_ETHEREUM => {
                        let address = parse_evm_address(&account.address, "turnkey_evm_address")?;
                        if !evm_addresses.contains(&address) {
                            evm_addresses.push(address);
                        }
                    }
                    ADDRESS_FORMAT_SOLANA => {
                        if !account.address.is_empty()
                            && !solana_addresses.contains(&account.address)
                        {
                            solana_addresses.push(account.address);
                        }
                    }
                    _ => {}
                }
            }

            if !evm_addresses.is_empty() && !solana_addresses.is_empty() {
                break;
            }
        }

        if evm_addresses.is_empty() {
            return Err(AppError::not_found(
                "No EVM address found in Turnkey wallets",
            ));
        }

        if solana_addresses.is_empty() {
            return Err(AppError::not_found(
                "No Solana address found in Turnkey wallets",
            ));
        }

        Ok(TurnkeyAddresses {
            evm_addresses,
            solana_addresses,
        })
    }

    async fn stamped_post<TReq, TRes>(&self, path: &str, payload: &TReq) -> Result<TRes, AppError>
    where
        TReq: Serialize + ?Sized,
        TRes: DeserializeOwned,
    {
        let body = serde_json::to_vec(payload)
            .map_err(|e| AppError::internal(format!("failed to encode Turnkey payload: {e}")))?;

        let stamp = self.build_stamp(&body)?;
        let url = format!(
            "{}/{}",
            self.turnkey_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let response = self
            .http
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Organization-Id", &self.turnkey_parent_org_id)
            .header("X-Stamp", stamp)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::network(format!(
                "Turnkey request failed: status={status}, body={body}"
            )));
        }

        response
            .json::<TRes>()
            .await
            .map_err(|e| AppError::network(format!("failed to decode Turnkey response: {e}")))
    }

    fn build_stamp(&self, body: &[u8]) -> Result<String, AppError> {
        let private_key_hex = self.turnkey_api_private_key.trim().trim_start_matches("0x");
        let private_key_bytes = hex::decode(private_key_hex)
            .map_err(|e| AppError::internal(format!("invalid TURNKEY_API_PRIVATE_KEY hex: {e}")))?;

        let key: [u8; 32] = private_key_bytes.try_into().map_err(|_| {
            AppError::internal("TURNKEY_API_PRIVATE_KEY must decode to exactly 32 bytes")
        })?;

        let signing_key = SigningKey::from_bytes((&key).into())
            .map_err(|e| AppError::internal(format!("invalid Turnkey private key bytes: {e}")))?;

        let signature: P256signature = signing_key.sign(body);
        let der_signature = signature.to_der();

        let stamp_payload = serde_json::json!({
            "publicKey": self.turnkey_api_public_key,
            "signature": hex::encode(der_signature.as_bytes()),
            "scheme": TURNKEY_SIGNATURE_SCHEME
        });

        let stamp_json = serde_json::to_vec(&stamp_payload)
            .map_err(|e| AppError::internal(format!("failed to encode stamp payload: {e}")))?;

        Ok(URL_SAFE_NO_PAD.encode(stamp_json))
    }
}

fn parse_challenge_message(message: &str) -> Result<ParsedChallengeMessage, AppError> {
    let parts: Vec<&str> = message.split('|').collect();
    if parts.len() != 5 || parts[0] != "nuketrade-login:v1" {
        return Err(AppError::unauthorised("invalid challenge message format"));
    }

    let suborg_id = parse_prefixed(parts[1], "suborg:")?.to_string();
    let nonce = parse_prefixed(parts[2], "nonce:")?.to_string();
    let issued_at_unix = parse_prefixed(parts[3], "issued_at:")?
        .parse::<u64>()
        .map_err(|_| AppError::unauthorised("invalid issued_at in challenge message"))?;
    let expires_at_unix = parse_prefixed(parts[4], "expires_at:")?
        .parse::<u64>()
        .map_err(|_| AppError::unauthorised("invalid expires_at in challenge message"))?;

    if Uuid::parse_str(&nonce).is_err() {
        return Err(AppError::unauthorised("invalid nonce in challenge message"));
    }

    if expires_at_unix <= issued_at_unix {
        return Err(AppError::unauthorised(
            "challenge expires_at must be greater than issued_at",
        ));
    }

    Ok(ParsedChallengeMessage {
        suborg_id,
        nonce,
        issued_at_unix,
        expires_at_unix,
    })
}

fn parse_prefixed<'a>(part: &'a str, prefix: &str) -> Result<&'a str, AppError> {
    part.strip_prefix(prefix)
        .ok_or_else(|| AppError::unauthorised("invalid challenge field"))
}

fn recover_evm_address(message: &str, signature: &str) -> Result<Address, AppError> {
    let mut sig_bytes = hex::decode(signature.trim().trim_start_matches("0x"))
        .map_err(|_| AppError::parse("signature", "signature must be valid hex"))?;

    if sig_bytes.len() != 65 {
        return Err(AppError::parse(
            "signature",
            "signature must be 65 bytes (r,s,v)",
        ));
    }

    if sig_bytes[64] >= 27 {
        sig_bytes[64] -= 27;
    }

    if sig_bytes[64] > 1 {
        return Err(AppError::parse(
            "signature",
            "invalid recovery id (v); expected 0/1 or 27/28",
        ));
    }

    let normalized = format!("0x{}", hex::encode(sig_bytes));

    let parsed = normalized
        .parse::<EvmSignature>()
        .map_err(|e| AppError::parse("signature", format!("invalid evm signature: {e}")))?;

    parsed
        .recover_address_from_msg(message.as_bytes())
        .map_err(|e| AppError::parse("signature", format!("failed to recover address: {e}")))
}

fn parse_evm_address(input: &str, field: &str) -> Result<Address, AppError> {
    input
        .parse::<Address>()
        .map_err(|e| AppError::parse(field, format!("invalid evm address: {e}")))
}

fn now_unix() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| AppError::internal(format!("system time error: {e}")))
}
