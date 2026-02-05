use perp_core::{ASSOCIATED_TOKEN_PROGRAM, Chain, SYSTEM_PROGRAM, TOKEN_PROGRAM};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use base64::{Engine, engine::general_purpose::STANDARD};
use bincode::serialize;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSimulateTransactionConfig;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_associated_token_account_client::address::get_associated_token_address;

use crate::{
    DEPOSIT_DISCRIMINATOR, EVENT_AUTHORITY, PACIFICA_CENTRAL_STATE_ADDRESS,
    PACIFICA_PROGRAM_ADDRESS, PACIFICA_VAULT_ADDRESS,
};

// 0.2 USDC = 200_000 (USDC has 6 decimals)
const GAS_REIMBURSEMENT_AMOUNT: u64 = 200_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositPayload {
    pub user_address: String,
    pub amount: u64,
}

pub async fn deposit_to_pacifica(
    solana_rpc_url: String,
    fee_payer_private_key: String,
    payload: DepositPayload,
) -> anyhow::Result<String> {
    let rpc = RpcClient::new(solana_rpc_url);

    let fee_payer_keypair = Keypair::from_base58_string(&fee_payer_private_key);
    let fee_payer_pubkey = fee_payer_keypair.pubkey();

    let user_pubkey = Pubkey::from_str(&payload.user_address)?;
    let usdc_pubkey = Pubkey::from_str(Chain::SOLANA.usdc_address)?;

    let user_usdc_ata = get_associated_token_address(&user_pubkey, &usdc_pubkey);
    let fee_payer_usdc_ata = get_associated_token_address(&fee_payer_pubkey, &usdc_pubkey);

    let central_state_pubkey = Pubkey::from_str(PACIFICA_CENTRAL_STATE_ADDRESS)?;
    let vault_pubkey = Pubkey::from_str(PACIFICA_VAULT_ADDRESS)?;

    let token_program_pubkey = Pubkey::from_str(TOKEN_PROGRAM)?;
    let system_program_pubkey = Pubkey::from_str(SYSTEM_PROGRAM)?;

    let associated_token_program_pubkey = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM)?;
    let event_authority_pubkey = Pubkey::from_str(EVENT_AUTHORITY)?;
    let pacifica_program_pubkey = Pubkey::from_str(PACIFICA_PROGRAM_ADDRESS)?;

    let amount_to_deposit = payload.amount - GAS_REIMBURSEMENT_AMOUNT;

    let deposit_ix = Instruction {
        program_id: pacifica_program_pubkey,
        accounts: vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(user_usdc_ata, false),
            AccountMeta::new(central_state_pubkey, false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new_readonly(token_program_pubkey, false),
            AccountMeta::new_readonly(associated_token_program_pubkey, false),
            AccountMeta::new_readonly(usdc_pubkey, false),
            AccountMeta::new_readonly(system_program_pubkey, false),
            AccountMeta::new_readonly(event_authority_pubkey, false),
            AccountMeta::new_readonly(pacifica_program_pubkey, false),
        ],
        data: {
            let mut d: Vec<u8> = Vec::with_capacity(16);
            d.extend_from_slice(&DEPOSIT_DISCRIMINATOR);
            d.extend_from_slice(&amount_to_deposit.to_le_bytes());
            d
        },
    };

    let gas_reimbursement_ix = Instruction {
        program_id: token_program_pubkey,
        accounts: vec![
            AccountMeta::new(user_usdc_ata, false),
            AccountMeta::new(fee_payer_usdc_ata, false),
            AccountMeta::new_readonly(user_pubkey, true),
        ],
        data: {
            let mut d: Vec<u8> = Vec::with_capacity(9);
            d.push(3); // Transfer instruction discriminator
            d.extend_from_slice(&GAS_REIMBURSEMENT_AMOUNT.to_le_bytes());
            d
        },
    };

    let message = Message::new(&[gas_reimbursement_ix, deposit_ix], Some(&fee_payer_pubkey));
    let transaction = Transaction::new_unsigned(message);

    let simulation_config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: Some(rpc.commitment()),
        ..Default::default()
    };

    let simulation_result = rpc
        .simulate_transaction_with_config(&transaction, simulation_config)
        .await?;

    if let Some(err) = simulation_result.value.err {
        log::error!("Deposit simulation failed: {:?}", err);
        anyhow::bail!("Deposit simulation failed: {:?}", err);
    }

    log::info!(
        "Deposit simulation successful. Units consumed: {:?}",
        simulation_result.value.units_consumed
    );

    if let Some(logs) = simulation_result.value.logs {
        for log in logs {
            log::info!("Log: {}", log);
        }
    }

    let serialized = serialize(&transaction)?;
    let encoded = STANDARD.encode(&serialized);

    Ok(encoded)
}
