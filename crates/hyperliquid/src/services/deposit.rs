use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolCall,
};
use perp_core::Chain;
use serde::{Deserialize, Serialize};

/// Hyperliquid deposit contract address (replace with actual)
pub const DEPOSIT_CONTRACT_ADDRESS: &str = "0x0000000000000000000000000000000000000000"; //TODO: change that

/// Minimum deposit amount: 10 USDC (6 decimals)
pub const MIN_DEPOSIT_AMOUNT: u64 = 10_000_000;

/// Permit signature components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitSignature {
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub deadline: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositPayload {
    pub amount: String,
    pub user: String,
    pub permit: PermitSignature,
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
}

impl std::fmt::Display for DepositError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepositError::InsufficientBalance {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient USDC balance: required {}, available {}",
                    required, available
                )
            }
            DepositError::BelowMinimumDeposit { amount, minimum } => {
                write!(
                    f,
                    "Deposit amount {} is below minimum {} USDC",
                    amount, minimum
                )
            }
            DepositError::SimulationFailed(msg) => write!(f, "Simulation failed: {}", msg),
            DepositError::ContractError(msg) => write!(f, "Contract error: {}", msg),
            DepositError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            DepositError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            DepositError::SignerError(msg) => write!(f, "Signer error: {}", msg),
            DepositError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
        }
    }
}

impl std::error::Error for DepositError {}

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }

    #[sol(rpc)]
    interface IDeposit {
        function deposit(address user, uint256 amount, uint256 deadline, uint8 v, bytes32 r, bytes32 s) external;
    }
}

//TODO: move to validation
/// Check if user has sufficient USDC balance
async fn check_user_balance(
    provider: &impl Provider,
    user_address: Address,
    required_amount: U256,
) -> Result<U256, DepositError> {
    let usdc_address: Address = Chain::ARBITRUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let usdc = IERC20::new(usdc_address, provider);

    let balance = usdc
        .balanceOf(user_address)
        .call()
        .await
        .map_err(|e| DepositError::ContractError(format!("{:?}", e)))?;

    if balance < required_amount {
        return Err(DepositError::InsufficientBalance {
            required: required_amount,
            available: balance,
        });
    }

    Ok(balance)
}

/// Encode the deposit call data
fn encode_deposit_call(
    user: Address,
    amount: u64,
    permit: &PermitSignature,
) -> Result<Bytes, DepositError> {
    let call = IDeposit::depositCall {
        user,
        amount: U256::from(amount),
        deadline: U256::from(permit.deadline),
        v: permit.v,
        r: FixedBytes::from(permit.r),
        s: FixedBytes::from(permit.s),
    };

    Ok(Bytes::from(call.abi_encode()))
}

/// Simulate the deposit transaction (called by fee payer)
async fn simulate_deposit(
    provider: &impl Provider,
    fee_payer: Address,
    user_address: Address,
    amount: u64,
    permit: &PermitSignature,
) -> Result<(), DepositError> {
    let contract_address: Address = DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let call_data = encode_deposit_call(user_address, amount, permit)?;

    let tx = TransactionRequest::default()
        .to(contract_address)
        .from(fee_payer)
        .input(call_data.into());

    provider
        .call(tx)
        .await
        .map_err(|e| DepositError::SimulationFailed(format!("{:?}", e)))?;

    log::info!(
        "Deposit simulation passed for user {} amount {} (fee payer: {})",
        user_address,
        amount,
        fee_payer
    );

    Ok(())
}

/// Main deposit function - fee payer calls the contract on behalf of user
///
/// 1. Checks if amount >= 10 USDC
/// 2. Checks if user has enough balance
/// 3. Simulates the transaction
/// 4. Executes the contract call and returns tx hash
pub async fn deposit_to_hyperliquid(
    arbitrum_rpc_url: &str,
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
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    // Check minimum deposit
    if amount < MIN_DEPOSIT_AMOUNT {
        return Err(DepositError::BelowMinimumDeposit {
            amount,
            minimum: MIN_DEPOSIT_AMOUNT,
        });
    }

    // Parse fee payer wallet
    let signer: PrivateKeySigner = fee_payer_private_key
        .parse()
        .map_err(|e| DepositError::SignerError(format!("{:?}", e)))?;

    let fee_payer_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(arbitrum_rpc_url)
        .await
        .map_err(|e| DepositError::ProviderError(format!("{:?}", e)))?;

    // Step 1: Check user balance
    let balance = check_user_balance(&provider, user_addr, U256::from(amount)).await?;
    log::info!(
        "User {} balance: {} (required: {})",
        user_addr,
        balance,
        payload.amount
    );

    // Step 2: Simulate the deposit
    simulate_deposit(
        &provider,
        fee_payer_address,
        user_addr,
        amount,
        &payload.permit,
    )
    .await?;

    // Step 3: Execute the contract call
    let contract_address: Address = DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let call_data = encode_deposit_call(user_addr, amount, &payload.permit)?;

    let tx = TransactionRequest::default()
        .to(contract_address)
        .from(fee_payer_address)
        .input(call_data.into());

    let pending_tx = provider
        .send_transaction(tx)
        .await
        .map_err(|e| DepositError::ContractError(format!("Failed to send tx: {:?}", e)))?;

    let tx_hash = *pending_tx.tx_hash();

    let receipt = pending_tx
        .get_receipt()
        .await
        .map_err(|e| DepositError::ContractError(format!("Tx failed: {:?}", e)))?;

    log::info!("Deposit successful! Tx hash: {}", tx_hash);

    Ok(format!("{:?}", receipt.transaction_hash))
}
