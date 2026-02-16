use std::{
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy::primitives::{Address, Signature as EvmSignature};
use anyhow::Context;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use p256::ecdsa::{Signature as P256Signature, SigningKey, signature::Signer};
use perp_core::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{error::AppError, features::auth::types::AuthClaims};

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
}

impl fmt::Debug for AuthService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthService")
            .field("turnkey_base_url", &self.turnkey_base_url)
            .field("turnkey_parent_org_id", &self.turnkey_parent_org_id)
            .field("jwt_ttl_secs", &self.jwt_ttl_secs)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct WalletAddresses {
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
        })
    }

    pub async fn login(
        &self,
        suborg_id: String,
        message: String,
        signature: String,
    ) -> Result<AuthLoginResult, AppError> {
        if suborg_id.is_empty() {
            return Err(AppError::parse("suborgId", "suborgId must not be empty"));
        }

        let recovered = recover_evm_address(&message, &signature)?;
        let wallets = self.fetch_turnkey_wallet_addresses(&suborg_id).await?;

        let matched_wallet = wallets
            .into_iter()
            .find(|wallet| wallet.evm_addresses.contains(&recovered))
            .ok_or_else(|| {
                AppError::unauthorised("signature address does not match any Turnkey EVM address")
            })?;

        let solana_address = matched_wallet
            .solana_addresses
            .first()
            .cloned()
            .ok_or_else(|| {
                AppError::not_found("No Solana address found in wallet matching EVM signer")
            })?;

        let evm_address = recovered.to_string();
        let (token, exp) =
            self.issue_jwt(suborg_id, evm_address.clone(), solana_address.clone())?;

        Ok(AuthLoginResult {
            token,
            evm_address,
            solana_address,
            expires_at_unix: exp,
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

    fn issue_jwt(
        &self,
        suborg_id: String,
        evm_address: String,
        solana_address: String,
    ) -> Result<(String, u64), AppError> {
        let now = now_unix()?;
        let exp = now.saturating_add(self.jwt_ttl_secs);

        let claims = AuthClaims {
            suborg_id,
            evm_address,
            solana_address,
            iat: now,
            exp,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::internal(format!("failed to issue jwt: {e}")))?;

        Ok((token, exp))
    }

    async fn fetch_turnkey_wallet_addresses(
        &self,
        suborg_id: &str,
    ) -> Result<Vec<WalletAddresses>, AppError> {
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

        let mut out = Vec::new();

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

            let mut evm_addresses = Vec::new();
            let mut solana_addresses = Vec::new();

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

            if !evm_addresses.is_empty() || !solana_addresses.is_empty() {
                out.push(WalletAddresses {
                    evm_addresses,
                    solana_addresses,
                });
            }
        }

        if out.is_empty() {
            return Err(AppError::not_found(
                "No EVM/Solana addresses found in Turnkey wallets",
            ));
        }

        Ok(out)
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

        let signature: P256Signature = signing_key.sign(body);
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

fn recover_evm_address(message: &str, signature: &str) -> Result<Address, AppError> {
    let signature = signature.trim();

    let parsed = signature
        .parse::<EvmSignature>()
        .or_else(|_| signature.trim_start_matches("0x").parse::<EvmSignature>())
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
