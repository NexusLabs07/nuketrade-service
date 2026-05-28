use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolCall,
};
use perp_core::{Chain, has_sufficient_balance};
use serde::{Deserialize, Serialize};

use crate::{LIGHTER_DEPOSIT_CONTRACT_ADDRESS, LIGHTER_ROUTE_TYPE_PERP, LIGHTER_USDC_ASSET_INDEX};

/// Minimum deposit amount per Lighter's docs: 1 USDC (6 decimals).
pub const MIN_DEPOSIT_AMOUNT: u64 = 1_000_000;

/// EIP-2612 permit signature components for USDC on Ethereum mainnet.
///
/// The user signs a permit authorising Lighter's bridge contract (the `spender`)
/// to pull `amount` USDC on their behalf. The fee payer then relays
/// `USDC.permit(...)` followed by `Lighter.deposit(...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitSignature {
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub deadline: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositPayload {
    /// USDC amount in 6-decimal base units.
    pub amount: String,
    /// Ethereum L1 address that will own the Lighter account (also the permit `owner`).
    pub user: String,
    pub permit: PermitSignature,
    /// Optional override for the asset index (defaults to USDC).
    #[serde(default)]
    pub asset_index: Option<u64>,
    /// Optional override for the route type (0 = perp, 1 = spot). Defaults to perp.
    #[serde(default)]
    pub route_type: Option<u64>,
}

#[derive(Debug)]
pub enum DepositError {
    InsufficientBalance { required: U256, available: U256 },
    BelowMinimumDeposit { amount: u64, minimum: u64 },
    SimulationFailed(String),
    ContractError(String),
    ProviderError(String),
    InvalidAddress(String),
    SignerError(String),
    InvalidAmount(String),
    PermitFailed(String),
}

impl std::fmt::Display for DepositError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepositError::InsufficientBalance {
                required,
                available,
            } => write!(
                f,
                "Insufficient USDC balance: required {required}, available {available}"
            ),
            DepositError::BelowMinimumDeposit { amount, minimum } => {
                write!(f, "Deposit amount {amount} is below minimum {minimum} USDC")
            }
            DepositError::SimulationFailed(msg) => write!(f, "Simulation failed: {msg}"),
            DepositError::ContractError(msg) => write!(f, "Contract error: {msg}"),
            DepositError::ProviderError(msg) => write!(f, "Provider error: {msg}"),
            DepositError::InvalidAddress(msg) => write!(f, "Invalid address: {msg}"),
            DepositError::SignerError(msg) => write!(f, "Signer error: {msg}"),
            DepositError::InvalidAmount(msg) => write!(f, "Invalid amount: {msg}"),
            DepositError::PermitFailed(msg) => write!(f, "Permit failed: {msg}"),
        }
    }
}

impl std::error::Error for DepositError {}

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 value) external returns (bool);
        function transferFrom(address from, address to, uint256 value) external returns (bool);
        function permit(
            address owner,
            address spender,
            uint256 value,
            uint256 deadline,
            uint8 v,
            bytes32 r,
            bytes32 s
        ) external;
    }

    #[sol(rpc)]
    interface ILighterDeposit {
        function deposit(
            address _to,
            uint16 _assetIndex,
            uint8 _routeType,
            uint256 _amount
        ) external payable;
    }
}

async fn check_user_balance(
    provider: &impl Provider,
    user_address: Address,
    required_amount: U256,
) -> Result<U256, DepositError> {
    let usdc_address: Address = Chain::ETHEREUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let usdc = IERC20::new(usdc_address, provider);

    let balance = usdc
        .balanceOf(user_address)
        .call()
        .await
        .map_err(|e| DepositError::ContractError(format!("{e:?}")))?;

    if !has_sufficient_balance(&balance, &required_amount) {
        return Err(DepositError::InsufficientBalance {
            required: required_amount,
            available: balance,
        });
    }

    Ok(balance)
}

fn encode_permit_call(
    owner: Address,
    spender: Address,
    amount: u64,
    permit: &PermitSignature,
) -> Result<Bytes, DepositError> {
    let call = IERC20::permitCall {
        owner,
        spender,
        value: U256::from(amount),
        deadline: U256::from(permit.deadline),
        v: permit.v,
        r: FixedBytes::from(permit.r),
        s: FixedBytes::from(permit.s),
    };

    Ok(Bytes::from(call.abi_encode()))
}

fn encode_deposit_call(
    user: Address,
    asset_index: u64,
    route_type: u64,
    amount: u64,
) -> Result<Bytes, DepositError> {
    let asset_index_u16: u16 = asset_index
        .try_into()
        .map_err(|_| DepositError::InvalidAmount(format!("asset_index {asset_index} exceeds u16")))?;
    let route_type_u8: u8 = route_type
        .try_into()
        .map_err(|_| DepositError::InvalidAmount(format!("route_type {route_type} exceeds u8")))?;

    let call = ILighterDeposit::depositCall {
        _to: user,
        _assetIndex: asset_index_u16,
        _routeType: route_type_u8,
        _amount: U256::from(amount),
    };

    Ok(Bytes::from(call.abi_encode()))
}

async fn simulate_deposit(
    provider: &impl Provider,
    fee_payer: Address,
    user: Address,
    asset_index: u64,
    route_type: u64,
    amount: u64,
) -> Result<(), DepositError> {
    let contract_address: Address = LIGHTER_DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let call_data = encode_deposit_call(user, asset_index, route_type, amount)?;

    let tx = TransactionRequest::default()
        .to(contract_address)
        .from(fee_payer)
        .input(call_data.into());

    provider
        .call(tx)
        .await
        .map_err(|e| DepositError::SimulationFailed(format!("{e:?}")))?;

    log::info!(
        "Lighter deposit simulation passed for user {user} amount {amount} (fee payer: {fee_payer})"
    );

    Ok(())
}

/// Submits `USDC.permit(user, fee_payer, amount, ...)` if the user has not
/// already granted enough allowance to the fee payer. The permit lets the
/// fee payer call `transferFrom(user, ...)` in the next step.
async fn ensure_permit(
    provider: &impl Provider,
    fee_payer: Address,
    user: Address,
    amount: u64,
    permit: &PermitSignature,
) -> Result<(), DepositError> {
    let usdc_address: Address = Chain::ETHEREUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let usdc = IERC20::new(usdc_address, provider);

    let allowance = usdc
        .allowance(user, fee_payer)
        .call()
        .await
        .map_err(|e| DepositError::ContractError(format!("allowance check failed: {e:?}")))?;

    if has_sufficient_balance(&allowance, &U256::from(amount)) {
        log::info!("user {user} already granted fee_payer {fee_payer} enough USDC allowance, skipping permit");
        return Ok(());
    }

    let call_data = encode_permit_call(user, fee_payer, amount, permit)?;

    let permit_tx = TransactionRequest::default()
        .to(usdc_address)
        .from(fee_payer)
        .input(call_data.into());

    let pending = provider
        .send_transaction(permit_tx)
        .await
        .map_err(|e| DepositError::PermitFailed(format!("send permit tx failed: {e:?}")))?;

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| DepositError::PermitFailed(format!("permit tx failed: {e:?}")))?;

    if !receipt.status() {
        return Err(DepositError::PermitFailed(format!(
            "USDC.permit reverted on-chain, tx: {:?}",
            receipt.transaction_hash
        )));
    }

    log::info!(
        "USDC permit executed for user {user} -> fee_payer {fee_payer} amount {amount}, tx: {:?}",
        receipt.transaction_hash
    );

    Ok(())
}

/// Pulls USDC from the user into the fee payer's wallet using the allowance
/// granted by the preceding `permit`. After this returns successfully the
/// fee payer holds `amount` USDC and the user holds `amount` less.
async fn pull_usdc_from_user(
    provider: &impl Provider,
    fee_payer: Address,
    user: Address,
    amount: u64,
) -> Result<(), DepositError> {
    let usdc_address: Address = Chain::ETHEREUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let call = IERC20::transferFromCall {
        from: user,
        to: fee_payer,
        value: U256::from(amount),
    };
    let call_data = Bytes::from(call.abi_encode());

    let tx = TransactionRequest::default()
        .to(usdc_address)
        .from(fee_payer)
        .input(call_data.into());

    let pending = provider
        .send_transaction(tx)
        .await
        .map_err(|e| DepositError::ContractError(format!("send transferFrom failed: {e:?}")))?;

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| DepositError::ContractError(format!("transferFrom tx failed: {e:?}")))?;

    if !receipt.status() {
        return Err(DepositError::ContractError(format!(
            "USDC.transferFrom(user, fee_payer, amount) reverted on-chain, tx: {:?}",
            receipt.transaction_hash
        )));
    }

    log::info!(
        "Pulled {amount} USDC from {user} into fee_payer {fee_payer}, tx: {:?}",
        receipt.transaction_hash
    );

    Ok(())
}

/// Ensures the fee payer has approved the Lighter bridge contract to spend
/// USDC. Done once when the allowance runs out; we approve `u256::MAX` so
/// this is effectively a one-time setup.
async fn ensure_fee_payer_allowance(
    provider: &impl Provider,
    fee_payer: Address,
    spender: Address,
    required_amount: u64,
) -> Result<(), DepositError> {
    let usdc_address: Address = Chain::ETHEREUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let usdc = IERC20::new(usdc_address, provider);

    let allowance = usdc
        .allowance(fee_payer, spender)
        .call()
        .await
        .map_err(|e| DepositError::ContractError(format!("fee_payer allowance check failed: {e:?}")))?;

    if has_sufficient_balance(&allowance, &U256::from(required_amount)) {
        return Ok(());
    }

    let call = IERC20::approveCall {
        spender,
        value: U256::MAX,
    };
    let call_data = Bytes::from(call.abi_encode());

    let tx = TransactionRequest::default()
        .to(usdc_address)
        .from(fee_payer)
        .input(call_data.into());

    let pending = provider
        .send_transaction(tx)
        .await
        .map_err(|e| DepositError::ContractError(format!("send approve failed: {e:?}")))?;

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| DepositError::ContractError(format!("approve tx failed: {e:?}")))?;

    if !receipt.status() {
        return Err(DepositError::ContractError(format!(
            "fee_payer USDC.approve(Lighter, MAX) reverted on-chain, tx: {:?}",
            receipt.transaction_hash
        )));
    }

    log::info!(
        "fee_payer {fee_payer} approved Lighter {spender} MAX USDC, tx: {:?}",
        receipt.transaction_hash
    );

    Ok(())
}

/// Main deposit function — fee payer relays a USDC permit and then invokes
/// Lighter's bridge deposit on Ethereum mainnet.
///
/// 1. Validates minimum deposit (≥ 1 USDC) and user's on-chain balance.
/// 2. Ensures allowance for the Lighter contract via `USDC.permit(...)` (only
///    if current allowance is insufficient).
/// 3. Simulates the deposit.
/// 4. Submits the deposit tx and returns the receipt hash.
pub async fn deposit_to_lighter(
    ethereum_rpc_url: &str,
    fee_payer_private_key: String,
    payload: DepositPayload,
) -> Result<String, DepositError> {
    let amount = payload
        .amount
        .parse::<u64>()
        .map_err(|e| DepositError::InvalidAmount(e.to_string()))?;

    let user_addr: Address = payload
        .user
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    if amount < MIN_DEPOSIT_AMOUNT {
        return Err(DepositError::BelowMinimumDeposit {
            amount,
            minimum: MIN_DEPOSIT_AMOUNT,
        });
    }

    let asset_index = payload.asset_index.unwrap_or(LIGHTER_USDC_ASSET_INDEX);
    let route_type = payload.route_type.unwrap_or(LIGHTER_ROUTE_TYPE_PERP);

    let signer: PrivateKeySigner = fee_payer_private_key
        .parse()
        .map_err(|e| DepositError::SignerError(format!("{e:?}")))?;

    let fee_payer_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(ethereum_rpc_url)
        .await
        .map_err(|e| DepositError::ProviderError(format!("{e:?}")))?;

    let spender: Address = LIGHTER_DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{e:?}")))?;

    let balance = check_user_balance(&provider, user_addr, U256::from(amount)).await?;
    log::info!(
        "Lighter deposit — user {} USDC balance: {} (required: {})",
        user_addr,
        balance,
        payload.amount
    );

    // Step 1: USDC.permit(user -> fee_payer). User's off-chain signature
    // gives the fee payer allowance to pull USDC out of the user's wallet.
    ensure_permit(&provider, fee_payer_address, user_addr, amount, &payload.permit).await?;

    // Step 2: fee_payer pulls USDC into its own wallet via transferFrom.
    // Lighter's deposit pulls from msg.sender, so the fee payer needs to own
    // the USDC at deposit time.
    pull_usdc_from_user(&provider, fee_payer_address, user_addr, amount).await?;

    // Step 3: ensure fee_payer has approved the Lighter bridge to spend its
    // USDC. Done once with MAX; subsequent deposits skip this.
    ensure_fee_payer_allowance(&provider, fee_payer_address, spender, amount).await?;

    // Step 4: simulate the deposit (msg.sender = fee_payer, _to = user).
    simulate_deposit(
        &provider,
        fee_payer_address,
        user_addr,
        asset_index,
        route_type,
        amount,
    )
    .await?;

    // Step 5: submit the actual deposit. Lighter pulls USDC from fee_payer
    // and credits user on L2.
    let call_data = encode_deposit_call(user_addr, asset_index, route_type, amount)?;

    let tx = TransactionRequest::default()
        .to(spender)
        .from(fee_payer_address)
        .input(call_data.into());

    let pending_tx = provider
        .send_transaction(tx)
        .await
        .map_err(|e| DepositError::ContractError(format!("Failed to send tx: {e:?}")))?;

    let tx_hash = *pending_tx.tx_hash();

    let receipt = pending_tx
        .get_receipt()
        .await
        .map_err(|e| DepositError::ContractError(format!("Tx failed: {e:?}")))?;

    if !receipt.status() {
        return Err(DepositError::ContractError(format!(
            "Lighter.deposit reverted on-chain, tx: {tx_hash:?}"
        )));
    }

    log::info!("Lighter deposit successful! Tx hash: {tx_hash}");

    Ok(format!("{:?}", receipt.transaction_hash))
}
