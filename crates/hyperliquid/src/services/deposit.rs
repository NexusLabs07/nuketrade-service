use ethers::{
    abi::{Function, Param, ParamType, Token},
    contract::Contract,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, Bytes, TransactionRequest, U256},
};
use perp_core::Chain;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Hyperliquid deposit contract address (replace with actual)
pub const DEPOSIT_CONTRACT_ADDRESS: &str = "0x0000000000000000000000000000000000000000"; //TODO: change that

/// Minimum deposit amount: 10 USDC (6 decimals)
pub const MIN_DEPOSIT_AMOUNT: u64 = 10_000_000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositPayload {
    pub amount: u64,
    pub user_address: String,
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
        }
    }
}

impl std::error::Error for DepositError {}

/// Permit signature components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitSignature {
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub deadline: U256,
}

/// Check if user has sufficient USDC balance
async fn check_user_balance(
    provider: &Provider<Http>,
    user_address: Address,
    required_amount: U256,
) -> Result<U256, DepositError> {
    let usdc_address: Address = Chain::ARBITRUM
        .usdc_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let abi: ethers::abi::Abi = serde_json::from_str(
        r#"[{
            "constant": true,
            "inputs": [{"name": "account", "type": "address"}],
            "name": "balanceOf",
            "outputs": [{"name": "", "type": "uint256"}],
            "type": "function"
        }]"#,
    )
    .expect("Invalid ABI");

    let usdc_contract = Contract::new(usdc_address, abi, Arc::new(provider.clone()));

    let balance: U256 = usdc_contract
        .method::<_, U256>("balanceOf", user_address)
        .map_err(|e| DepositError::ContractError(format!("{:?}", e)))?
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

/// Encode the depositWithPermit call data
fn encode_deposit_with_permit_call(
    amount: u64,
    permit: &PermitSignature,
) -> Result<Bytes, DepositError> {
    // depositWithPermit(uint256 amount, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
    let deposit_fn = Function {
        name: "depositWithPermit".to_string(),
        inputs: vec![
            Param {
                name: "amount".to_string(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
            Param {
                name: "deadline".to_string(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
            Param {
                name: "v".to_string(),
                kind: ParamType::Uint(8),
                internal_type: None,
            },
            Param {
                name: "r".to_string(),
                kind: ParamType::FixedBytes(32),
                internal_type: None,
            },
            Param {
                name: "s".to_string(),
                kind: ParamType::FixedBytes(32),
                internal_type: None,
            },
        ],
        outputs: vec![],
        constant: None,
        state_mutability: ethers::abi::StateMutability::NonPayable,
    };

    let tokens = vec![
        Token::Uint(U256::from(amount)),
        Token::Uint(permit.deadline),
        Token::Uint(U256::from(permit.v)),
        Token::FixedBytes(permit.r.to_vec()),
        Token::FixedBytes(permit.s.to_vec()),
    ];

    let encoded = deposit_fn
        .encode_input(&tokens)
        .map_err(|e| DepositError::ContractError(format!("Failed to encode: {:?}", e)))?;

    Ok(Bytes::from(encoded))
}

/// Simulate the deposit transaction (called by fee payer)
async fn simulate_deposit<M: Middleware>(
    client: &M,
    fee_payer: Address,
    user_address: Address,
    amount: u64,
    permit: &PermitSignature,
) -> Result<(), DepositError> {
    let contract_address: Address = DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let call_data = encode_deposit_with_permit_call(amount, permit)?;

    let tx = TransactionRequest::new()
        .to(contract_address)
        .from(fee_payer)
        .data(call_data);

    client
        .call(&tx.into(), None)
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
    // Check minimum deposit
    if payload.amount < MIN_DEPOSIT_AMOUNT {
        return Err(DepositError::BelowMinimumDeposit {
            amount: payload.amount,
            minimum: MIN_DEPOSIT_AMOUNT,
        });
    }

    let provider = Provider::<Http>::try_from(arbitrum_rpc_url)
        .map_err(|e| DepositError::ProviderError(format!("{:?}", e)))?;

    let user_addr: Address = payload
        .user_address
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    // Parse fee payer wallet
    let fee_payer_wallet: LocalWallet = fee_payer_private_key
        .parse::<LocalWallet>()
        .map_err(|e| DepositError::SignerError(format!("{:?}", e)))?
        .with_chain_id(Chain::ARBITRUM.id);

    let fee_payer_address = fee_payer_wallet.address();

    // Step 1: Check user balance
    let balance = check_user_balance(&provider, user_addr, U256::from(payload.amount)).await?;
    log::info!(
        "User {} balance: {} (required: {})",
        user_addr,
        balance,
        payload.amount
    );

    // Step 2: Simulate the deposit
    let client = SignerMiddleware::new(provider.clone(), fee_payer_wallet);
    simulate_deposit(
        &client,
        fee_payer_address,
        user_addr,
        payload.amount,
        &payload.permit,
    )
    .await?;

    // Step 3: Execute the contract call
    let contract_address: Address = DEPOSIT_CONTRACT_ADDRESS
        .parse()
        .map_err(|e| DepositError::InvalidAddress(format!("{:?}", e)))?;

    let call_data = encode_deposit_with_permit_call(payload.amount, &payload.permit)?;

    let tx = TransactionRequest::new()
        .to(contract_address)
        .from(fee_payer_address)
        .data(call_data)
        .chain_id(Chain::ARBITRUM.id);

    let pending_tx = client
        .send_transaction(tx, None)
        .await
        .map_err(|e| DepositError::ContractError(format!("Failed to send tx: {:?}", e)))?;

    let receipt = pending_tx
        .await
        .map_err(|e| DepositError::ContractError(format!("Tx failed: {:?}", e)))?
        .ok_or_else(|| DepositError::ContractError("No receipt".to_string()))?;

    let tx_hash = format!("{:?}", receipt.transaction_hash);
    log::info!("Deposit successful! Tx hash: {}", tx_hash);

    Ok(tx_hash)
}
