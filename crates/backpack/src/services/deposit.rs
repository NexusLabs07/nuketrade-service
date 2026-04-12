use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use bincode::serialize;
use ed25519_dalek::{Signer, SigningKey};
use perp_core::{Chain, TOKEN_PROGRAM, has_sufficient_balance};
use serde::{Deserialize, Serialize};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSimulateTransactionConfig;
use solana_signer::Signer as SolanaSigner;
use solana_transaction::Transaction;
use spl_associated_token_account_client::address::get_associated_token_address;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const MINIMUM_DEPOSIT_AMOUNT: u64 = 1_000_000; // 1 USDC (6 decimals)
const BACKPACK_SIGNATURE_WINDOW: &str = "5000";
const BACKPACK_DEPOSIT_ADDRESS_URL: &str =
    "https://api.backpack.exchange/wapi/v1/capital/deposit/address";

// ============================= Request Types =============================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositPayload {
    pub user_address: String,
    /// Amount in USDC raw units (6 decimals). e.g. 10_000_000 = 10 USDC.
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
struct DepositAddressResponse {
    address: String,
}

// ============================= Signing =============================

/// Build the Backpack ED25519 signature for a GET request.
///
/// Backpack's signing scheme:
/// 1. Collect all query params + `instruction` + `timestamp` + `window`
/// 2. Sort keys alphabetically, join as `key=value&...`
/// 3. Sign the resulting string with the ED25519 secret key
/// 4. Base64-encode the 64-byte signature
fn sign_backpack_get(
    api_secret: &str,
    instruction: &str,
    extra_params: &[(&str, &str)],
) -> anyhow::Result<(String, String)> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time error")?
        .as_millis()
        .to_string();

    // Build the sorted parameter list.
    let mut params: Vec<(&str, &str)> = extra_params.to_vec();
    params.push(("instruction", instruction));
    params.push(("timestamp", &timestamp));
    params.push(("window", BACKPACK_SIGNATURE_WINDOW));
    params.sort_by_key(|(k, _)| *k);

    let message = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    // Decode the base64 API secret (raw 32-byte ED25519 seed).
    let secret_bytes = STANDARD
        .decode(api_secret)
        .context("Failed to base64-decode BACKPACK_API_SECRET")?;
    let secret_array: [u8; 32] = secret_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("BACKPACK_API_SECRET must be 32 bytes after base64 decode"))?;

    let signing_key = SigningKey::from_bytes(&secret_array);
    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = STANDARD.encode(signature.to_bytes());

    Ok((timestamp, signature_b64))
}

// ============================= Internal Helpers =============================

/// Fetch the Backpack USDC deposit wallet address via their authenticated API.
async fn fetch_deposit_address(api_key: &str, api_secret: &str) -> anyhow::Result<String> {
    let (timestamp, signature) =
        sign_backpack_get(api_secret, "depositAddressQuery", &[("blockchain", "Solana")])?;

    let client = reqwest::Client::new();
    let response = client
        .get(BACKPACK_DEPOSIT_ADDRESS_URL)
        .query(&[("blockchain", "Solana")])
        .header("X-API-KEY", api_key)
        .header("X-SIGNATURE", &signature)
        .header("X-TIMESTAMP", &timestamp)
        .header("X-WINDOW", BACKPACK_SIGNATURE_WINDOW)
        .send()
        .await
        .context("Failed to call Backpack depositAddressQuery API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Backpack deposit address API returned HTTP {status}: {body}");
    }

    let data: DepositAddressResponse = response
        .json()
        .await
        .context("Failed to parse Backpack deposit address response")?;

    Ok(data.address)
}

// ============================= Public API =============================

/// Build and partially-sign a Solana USDC transfer transaction to Backpack's deposit address.
///
/// Flow:
/// 1. Signs the Backpack API request server-side (api_key + api_secret from env).
/// 2. Fetches the deposit wallet address from Backpack's capital API.
/// 3. Derives user's USDC ATA and the deposit USDC ATA.
/// 4. Builds an SPL token transfer: user_ata → deposit_ata, owner = user.
/// 5. Simulates (3 retries), partial-signs with the fee payer.
/// 6. Returns a base64-encoded serialized transaction for the client to co-sign and submit.
pub async fn deposit_to_backpack(
    solana_rpc_url: String,
    fee_payer_private_key: String,
    api_key: String,
    api_secret: String,
    payload: DepositPayload,
) -> anyhow::Result<String> {
    log::info!(
        "Starting Backpack deposit: user={}, amount={}",
        payload.user_address,
        payload.amount
    );

    // ── 1. Get deposit address from Backpack ─────────────────────────────
    let deposit_wallet_address = fetch_deposit_address(&api_key, &api_secret).await?;
    log::info!("Backpack deposit wallet address: {deposit_wallet_address}");

    // ── 2. Resolve keys and ATAs ──────────────────────────────────────────
    let rpc = RpcClient::new(solana_rpc_url);

    let fee_payer_keypair = Keypair::from_base58_string(&fee_payer_private_key);
    let fee_payer_pubkey = fee_payer_keypair.pubkey();

    let user_pubkey =
        Pubkey::from_str(&payload.user_address).context("Invalid user Solana address")?;
    let deposit_pubkey =
        Pubkey::from_str(&deposit_wallet_address).context("Invalid Backpack deposit address")?;
    let usdc_pubkey =
        Pubkey::from_str(Chain::SOLANA.usdc_address).context("Invalid USDC mint address")?;

    let user_usdc_ata = get_associated_token_address(&user_pubkey, &usdc_pubkey);
    let deposit_usdc_ata = get_associated_token_address(&deposit_pubkey, &usdc_pubkey);

    log::info!("ATAs — user: {user_usdc_ata}, deposit: {deposit_usdc_ata}");

    // ── 3. Check user balance ─────────────────────────────────────────────
    let user_usdc_balance = rpc
        .get_token_account_balance_with_commitment(&user_usdc_ata, CommitmentConfig::confirmed())
        .await
        .map(|resp| resp.value.amount.parse::<u64>().unwrap_or(0))
        .context("Failed to get user USDC balance from Solana RPC")?;

    log::info!("User USDC balance: {user_usdc_balance}");

    if !has_sufficient_balance(&user_usdc_balance, &MINIMUM_DEPOSIT_AMOUNT) {
        anyhow::bail!(
            "Insufficient USDC balance. Minimum: {MINIMUM_DEPOSIT_AMOUNT}, Balance: {user_usdc_balance}"
        );
    }

    if !has_sufficient_balance(&user_usdc_balance, &payload.amount) {
        anyhow::bail!(
            "Insufficient USDC balance. Required: {}, Balance: {user_usdc_balance}",
            payload.amount
        );
    }

    // ── 4. Build SPL token transfer instruction ───────────────────────────
    let token_program_pubkey =
        Pubkey::from_str(TOKEN_PROGRAM).context("Invalid TOKEN_PROGRAM address")?;

    let transfer_ix = Instruction {
        program_id: token_program_pubkey,
        accounts: vec![
            AccountMeta::new(user_usdc_ata, false),       // source ATA (writable)
            AccountMeta::new(deposit_usdc_ata, false),    // destination ATA (writable)
            AccountMeta::new_readonly(user_pubkey, true), // owner / signer
        ],
        data: {
            let mut d: Vec<u8> = Vec::with_capacity(9);
            d.push(3); // SPL Transfer discriminator
            d.extend_from_slice(&payload.amount.to_le_bytes());
            d
        },
    };

    let message = Message::new(&[transfer_ix], Some(&fee_payer_pubkey));
    let mut transaction = Transaction::new_unsigned(message);

    // ── 5. Simulate (3 retries) ───────────────────────────────────────────
    const MAX_SIMULATION_RETRIES: u64 = 3;
    let mut simulation_result = None;
    let mut last_err = None;

    for attempt in 1..=MAX_SIMULATION_RETRIES {
        let config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };

        let result = rpc
            .simulate_transaction_with_config(&transaction, config)
            .await?;

        if let Some(err) = result.value.err {
            log::warn!(
                "Backpack deposit simulation attempt {attempt}/{MAX_SIMULATION_RETRIES} failed: {err:?}"
            );
            if let Some(logs) = &result.value.logs {
                for line in logs {
                    log::warn!("  {line}");
                }
            }
            last_err = Some(err);
            if attempt < MAX_SIMULATION_RETRIES {
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt)).await;
            }
            continue;
        }

        simulation_result = Some(result);
        break;
    }

    let simulation_result = match simulation_result {
        Some(r) => r,
        None => {
            let err = last_err.expect("last_err set after failed retries");
            log::error!(
                "Backpack deposit simulation failed after all {MAX_SIMULATION_RETRIES} retries: {err:?}"
            );
            anyhow::bail!("Backpack deposit simulation failed: {err:?}");
        }
    };

    log::info!(
        "Backpack deposit simulation successful. Units consumed: {:?}",
        simulation_result.value.units_consumed
    );

    // ── 6. Partial sign and serialize ─────────────────────────────────────
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    transaction.partial_sign(&[&fee_payer_keypair], recent_blockhash);

    let serialized = serialize(&transaction)?;
    let encoded = STANDARD.encode(&serialized);

    Ok(encoded)
}
