//! Balance Checking Service
//!
//! Queries existing USDC balances across protocols and chains so the hedge
//! intent system can skip redundant bridges/deposits.
//!
//! All queries are **read-only** — no signing required. Errors are treated as
//! "unknown balance = 0" so a failed query never blocks the flow, it just means
//! we do a full bridge + deposit instead of skipping.

use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    sol,
};
use perp_core::{Chain, config::Config};
use serde::Deserialize;

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

/// USDC has 6 decimals.
const USDC_DECIMALS: f64 = 1_000_000.0;

// ============================= Public Types =============================

/// Pre-existing balances for a single hedge leg.
#[derive(Debug, Clone, Default)]
pub struct LegBalances {
    /// USDC already inside the exchange's margin account.
    pub exchange_margin_used: f64,
    /// USDC on the destination chain (wallet balance, not yet deposited).
    pub onchain_usd: f64,
}

// ============================= HL Balance Checks =============================

/// Query Hyperliquid margin balance via clearinghouseState API (read-only, no signing).
async fn query_hl_margin_balance(evm_address: &str) -> Result<f64, anyhow::Error> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "type": "clearinghouseState",
        "user": evm_address,
    });

    let response = client
        .post("https://api.hyperliquid.xyz/info")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Hyperliquid clearinghouseState returned HTTP {}",
            response.status()
        );
    }

    let state: HlClearinghouseResponse = response.json().await?;

    // `withdrawable` represents available USDC not locked in positions.
    let withdrawable = state.withdrawable.parse::<f64>().unwrap_or(0.0);

    Ok(withdrawable)
}

/// Query on-chain USDC balance on Arbitrum via ERC20 balanceOf.
async fn query_arb_onchain_usdc(config: &Config, evm_address: &str) -> Result<f64, anyhow::Error> {
    let user_address: Address = evm_address
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid EVM address: {e:?}"))?;

    let usdc_address: Address = Chain::ARBITRUM
        .usdc_address
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid USDC address: {e:?}"))?;

    let provider = ProviderBuilder::new().connect_http(config.arbitrum_rpc_url.parse()?);
    let usdc = IERC20::new(usdc_address, provider);

    let balance: U256 = usdc
        .balanceOf(user_address)
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("Arb balanceOf failed: {e:?}"))?;

    let balance_usd = balance.to::<u128>() as f64 / USDC_DECIMALS;
    Ok(balance_usd)
}

// ============================= Pacifica Balance Checks =============================

/// Query Pacifica margin/collateral balance via Pacifica API.
///
/// Tries `GET /account/collateral?account=<address>`. Falls back to 0 if the
/// endpoint doesn't exist or returns an error (Pacifica may not expose this yet).
async fn query_pacifica_margin_balance(solana_address: &str) -> Result<f64, anyhow::Error> {
    let client = reqwest::Client::new();

    // Try the collateral endpoint
    let url = format!("https://api.pacifica.fi/api/v1/account/collateral?account={solana_address}");

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        // Endpoint might not exist — fall back to 0.
        log::debug!(
            "Pacifica collateral endpoint returned {}, falling back to 0",
            response.status()
        );
        return Ok(0.0);
    }

    let data: PacificaCollateralResponse = match response.json().await {
        Ok(d) => d,
        Err(_) => return Ok(0.0),
    };

    if !data.success {
        return Ok(0.0);
    }

    Ok(data
        .data
        .and_then(|d| d.collateral.parse::<f64>().ok())
        .unwrap_or(0.0))
}

/// Query on-chain USDC balance on Solana via RPC `getTokenAccountsByOwner`.
///
/// Uses a raw JSON-RPC call so we don't need the full Solana SDK in the server crate.
async fn query_sol_onchain_usdc(
    config: &Config,
    solana_address: &str,
) -> Result<f64, anyhow::Error> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            solana_address,
            { "mint": Chain::SOLANA.usdc_address },
            { "encoding": "jsonParsed" }
        ]
    });

    let response = client
        .post(&config.solana_rpc_url)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Solana RPC returned HTTP {}", response.status());
    }

    let rpc_response: SolanaRpcResponse = response.json().await?;

    // Sum up USDC balances across all token accounts for this mint.
    let total_usd: f64 = rpc_response
        .result
        .value
        .iter()
        .filter_map(|account| {
            let info = account.account.get("data")?.get("parsed")?.get("info")?;

            let amount_str = info.get("tokenAmount")?.get("uiAmountString")?.as_str()?;

            amount_str.parse::<f64>().ok()
        })
        .sum();

    Ok(total_usd)
}

// ============================= Public API =============================
/// Query all relevant balances for a Hyperliquid leg.
pub async fn check_hl_balances(config: &Config, evm_address: &str) -> LegBalances {
    let margin = query_hl_margin_balance(evm_address)
        .await
        .unwrap_or_else(|e| {
            log::warn!("Failed to query Hyperliquid margin balance: {e}, defaulting to 0");
            0.0
        });

    let onchain = query_arb_onchain_usdc(config, evm_address)
        .await
        .unwrap_or_else(|e| {
            log::warn!("Failed to query Arb on-chain USDC balance: {e}, defaulting to 0");
            0.0
        });

    log::info!(
        "Hyperliquid balance check for {evm_address}: margin={margin:.2}, on-chain={onchain:.2}"
    );

    LegBalances {
        exchange_margin_used: margin,
        onchain_usd: onchain,
    }
}

/// Query all relevant balances for a Pacifica leg.
pub async fn check_pacifica_balances(config: &Config, solana_address: &str) -> LegBalances {
    let margin = query_pacifica_margin_balance(solana_address)
        .await
        .unwrap_or_else(|e| {
            log::warn!("Failed to query Pacifica margin balance: {e}, defaulting to 0");
            0.0
        });

    let onchain = query_sol_onchain_usdc(config, solana_address)
        .await
        .unwrap_or_else(|e| {
            log::warn!("Failed to query Solana on-chain USDC balance: {e}, defaulting to 0");
            0.0
        });

    log::info!(
        "Pacifica balance check for {solana_address}: margin={margin:.2}, on-chain={onchain:.2}"
    );

    LegBalances {
        exchange_margin_used: margin,
        onchain_usd: onchain,
    }
}

//TODO: make this dynamic
/// Query balances for a leg based on its protocol.
pub async fn check_leg_balances(
    config: &Config,
    exchange: &str,
    evm_address: &str,
    solana_address: &str,
) -> LegBalances {
    match exchange {
        "hyperliquid" => check_hl_balances(config, evm_address).await,
        "pacifica" => check_pacifica_balances(config, solana_address).await,
        _ => {
            log::warn!("Unknown exchange {exchange}, returning zero balances");
            LegBalances::default()
        }
    }
}

/// Compute how much needs to be bridged and deposited for a leg given its existing balances.
pub fn compute_funding_needs(
    target_amount_usd: f64,
    existing_margin_usd: f64,
    existing_onchain_usd: f64,
) -> FundingNeeds {
    let deposit_needed = (target_amount_usd - existing_margin_usd).max(0.0);
    let bridge_needed = (deposit_needed - existing_onchain_usd).max(0.0);

    FundingNeeds {
        deposit_needed,
        bridge_needed,
    }
}

/// Computed funding needs for a leg.
#[derive(Debug, Clone)]
pub struct FundingNeeds {
    /// How much USDC needs to be deposited into the protocol margin.
    pub deposit_needed: f64,
    /// How much USDC needs to be bridged from Base to the destination chain.
    pub bridge_needed: f64,
}

// ============================= Internal Response Types =============================

/// Hyperliquid clearinghouseState response (minimal fields we need).
#[derive(Debug, Deserialize)]
struct HlClearinghouseResponse {
    withdrawable: String,
}

/// Pacifica collateral response (best-effort).
#[derive(Debug, Deserialize)]
struct PacificaCollateralResponse {
    success: bool,
    data: Option<PacificaCollateralData>,
}

#[derive(Debug, Deserialize)]
struct PacificaCollateralData {
    collateral: String,
}

/// Solana JSON-RPC response for getTokenAccountsByOwner.
#[derive(Debug, Deserialize)]
struct SolanaRpcResponse {
    result: SolanaRpcResult,
}

#[derive(Debug, Deserialize)]
struct SolanaRpcResult {
    value: Vec<SolanaTokenAccount>,
}

#[derive(Debug, Deserialize)]
struct SolanaTokenAccount {
    account: serde_json::Value,
}
