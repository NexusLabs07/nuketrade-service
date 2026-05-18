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

use db::user::{upsert_google_user, upsert_wallet_user};
use uuid::Uuid;

use crate::{
    error::AppError,
    features::auth::types::{AuthClaims, GoogleIdClaims},
};

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
    google_client_id: String,
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
pub struct VerifySignatureResponse {
    pub evm_address: String,
    pub solana_address: String,
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
            google_client_id: config.google_client_id.clone(),
        })
    }

    pub async fn login(
        &self,
        suborg_id: String,
        message: String,
        signature: String,
    ) -> Result<VerifySignatureResponse, AppError> {
        if suborg_id.is_empty() {
            return Err(AppError::parse("suborgId", "suborgId must not be empty"));
        }

        //recover the evm address from the signature
        let recovered = recover_evm_address(&message, &signature)?;

        //fetch turnkey wallets
        let wallets = self.fetch_turnkey_wallet_addresses(&suborg_id).await?;

        //check if wallet matched and throw error if not
        let matched_wallet = wallets
            .into_iter()
            .find(|wallet| wallet.evm_addresses.contains(&recovered))
            .ok_or_else(|| {
                AppError::unauthorised("signature address does not match any Turnkey EVM address")
            })?;

        //TODO: This only handles the case where only 1 solana address is associated with the matched wallet - we may want to support multiple in future
        let solana_address = matched_wallet
            .solana_addresses
            .first()
            .cloned()
            .ok_or_else(|| {
                AppError::not_found("No Solana address found in wallet matching EVM signer")
            })?;

        let evm_address = recovered.to_string();

        Ok(VerifySignatureResponse {
            evm_address,
            solana_address,
        })
    }

    pub async fn google_login(
        &self,
        db: std::sync::Arc<sqlx::PgPool>,
        id_token: String,
        evm_address: String,
        solana_address: String,
    ) -> Result<(Uuid, Uuid), AppError> {
        #[derive(Debug, Deserialize)]
        struct Jwk {
            kid: String,
            n: String,
            e: String,
        }

        #[derive(Debug, Deserialize)]
        struct JwkSet {
            keys: Vec<Jwk>,
        }

        // Decode header (unverified) to get the key ID
        let header = jsonwebtoken::decode_header(&id_token)
            .map_err(|_| AppError::unauthorised("invalid Google ID token header"))?;
        let kid = header
            .kid
            .ok_or_else(|| AppError::unauthorised("Google ID token missing kid"))?;

        // Fetch Google's public keys
        let jwks: JwkSet = self
            .http
            .get("https://www.googleapis.com/oauth2/v3/certs")
            .send()
            .await?
            .json()
            .await
            .map_err(|e| AppError::network(format!("failed to parse Google JWKS: {e}")))?;

        let jwk = jwks
            .keys
            .iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| AppError::unauthorised("no matching Google public key for kid"))?;

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|e| AppError::internal(format!("failed to build Google decoding key: {e}")))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.google_client_id]);
        validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);

        let token_data = decode::<GoogleIdClaims>(&id_token, &decoding_key, &validation)
            .map_err(|_| AppError::unauthorised("invalid or expired Google ID token"))?;

        let google_claims = token_data.claims;

        // Upsert wallet then user into the DB
        let (wallet_id, user) = upsert_google_user(
            db,
            google_claims.email.clone(),
            google_claims.name.clone(),
            evm_address,
            solana_address,
        )
        .await
        .map_err(|e| AppError::internal(format!("failed to upsert Google user: {e}")))?;

        Ok((wallet_id, user.id))
    }

    pub async fn wallet_login(
        &self,
        db: std::sync::Arc<sqlx::PgPool>,
        evm_address: String,
        solana_address: String,
    ) -> Result<(Uuid, Uuid), AppError> {
        let (wallet_id, user) = upsert_wallet_user(db, evm_address, solana_address)
            .await
            .map_err(|e| AppError::internal(format!("failed to upsert wallet user: {e}")))?;

        Ok((wallet_id, user.id))
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

    pub fn issue_jwt(
        &self,
        suborg_id: String,
        user_id: String,
        wallet_id: String,
        evm_address: String,
        solana_address: String,
    ) -> Result<(String, u64), AppError> {
        let now = now_unix()?;
        let exp = now.saturating_add(self.jwt_ttl_secs);

        let claims = AuthClaims {
            suborg_id,
            user_id,
            wallet_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::{Signer, local::PrivateKeySigner};

    fn test_auth_service(jwt_secret: &str) -> AuthService {
        AuthService {
            http: Client::new(),
            turnkey_base_url: "https://api.turnkey.com".into(),
            turnkey_parent_org_id: "test-org".into(),
            turnkey_api_public_key: "test-pub".into(),
            turnkey_api_private_key: "a".repeat(64),
            jwt_secret: jwt_secret.into(),
            jwt_ttl_secs: 3600,
            google_client_id: "test-google-client-id".into(),
        }
    }

    #[test]
    fn issue_and_verify_jwt_round_trip() {
        let svc = test_auth_service("test-secret-key-12345");
        let (token, exp) = svc
            .issue_jwt(
                "sub-org-1".into(),
                "user-123".into(),
                "wallet-456".into(),
                "0xABCD".into(),
                "SoLaNaAddr".into(),
            )
            .unwrap();

        assert!(!token.is_empty());
        assert!(exp > 0);

        let claims = svc.verify_token(&token).unwrap();
        assert_eq!(claims.suborg_id, "sub-org-1");
        assert_eq!(claims.user_id, "user-123");
        assert_eq!(claims.wallet_id, "wallet-456");
        assert_eq!(claims.evm_address, "0xABCD");
        assert_eq!(claims.solana_address, "SoLaNaAddr");
        assert_eq!(claims.exp, exp);
    }

    #[test]
    fn verify_token_rejects_garbage() {
        let svc = test_auth_service("my-secret");
        let err = svc.verify_token("not.a.real.token").unwrap_err();
        assert!(
            format!("{err}").contains("invalid or expired"),
            "expected unauthorised, got: {err}"
        );
    }

    #[test]
    fn verify_token_rejects_wrong_secret() {
        let svc_a = test_auth_service("secret-a");
        let svc_b = test_auth_service("secret-b");

        let (token_a, _) = svc_a
            .issue_jwt(
                "org".into(),
                "user-1".into(),
                "wallet-1".into(),
                "0x1".into(),
                "sol1".into(),
            )
            .unwrap();

        let err = svc_b.verify_token(&token_a).unwrap_err();
        assert!(
            format!("{err}").contains("invalid or expired"),
            "expected unauthorised, got: {err}"
        );
    }

    #[test]
    fn verify_token_rejects_expired() {
        let svc = AuthService {
            jwt_ttl_secs: 0, // expires immediately
            ..test_auth_service("expire-test")
        };

        // Craft a manually expired token (leeway is 30s so ttl=0 alone won't expire fast enough):
        let expired_claims = AuthClaims {
            suborg_id: "org".into(),
            user_id: "user-1".into(),
            wallet_id: "wallet-1".into(),
            evm_address: "0x1".into(),
            solana_address: "sol1".into(),
            iat: 1000,
            exp: 1001, // far in the past
        };
        let expired_token = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret(b"expire-test"),
        )
        .unwrap();

        let err = svc.verify_token(&expired_token).unwrap_err();
        assert!(
            format!("{err}").contains("invalid or expired"),
            "expected unauthorised, got: {err}"
        );
    }

    #[test]
    fn parse_evm_address_valid() {
        let addr = parse_evm_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", "test").unwrap();
        assert_eq!(
            addr,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
                .parse::<Address>()
                .unwrap()
        );
    }

    #[test]
    fn parse_evm_address_invalid() {
        let err = parse_evm_address("not-an-address", "field").unwrap_err();
        assert!(format!("{err}").contains("invalid evm address"));
    }

    #[tokio::test]
    async fn recover_evm_address_valid_signature() {
        let signer = PrivateKeySigner::random();
        let message = "hello world";
        let signature = signer.sign_message(message.as_bytes()).await.unwrap();
        let sig_hex = format!("0x{}", hex::encode(signature.as_bytes()));

        let recovered = recover_evm_address(message, &sig_hex).unwrap();
        assert_eq!(recovered, signer.address());
    }

    #[test]
    fn recover_evm_address_invalid_signature() {
        let err = recover_evm_address("msg", "0xdeadbeef").unwrap_err();
        assert!(format!("{err}").contains("invalid evm signature"));
    }

    #[test]
    fn now_unix_returns_reasonable_value() {
        let ts = now_unix().unwrap();
        // Should be after 2024-01-01
        assert!(ts > 1_704_067_200);
    }
}
