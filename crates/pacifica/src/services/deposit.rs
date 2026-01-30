use core::{ASSOCIATED_TOKEN_PROGRAM, SOLANA_USDC_MINT, SYSTEM_PROGRAM, TOKEN_PROGRAM};
use std::str::FromStr;

use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use spl_associated_token_account_client::address::get_associated_token_address;

use crate::services::{
    DEPOSIT_DISCRIMINATOR, EVENT_AUTHORITY, PACIFICA_CENTRAL_STATE_ADDRESS,
    PACIFICA_PROGRAM_ADDRESS, PACIFICA_VAULT_ADDRESS,
};

pub async fn deposit_to_pacifica(solana_rpc_url: String, depositor: String, amount: u64) {
    let rpc = RpcClient::new(solana_rpc_url);

    let payer_keypair = Keypair::from_base58_string("FEE_PAYER_PRIVATE_KEY");
    let user_keypair = Keypair::from_base58_string(&depositor);

    let user_pubkey = Pubkey::from_str(&depositor).unwrap();
    let usdc_pubkey = Pubkey::from_str(&SOLANA_USDC_MINT).unwrap();

    let user_usdc_ata = get_associated_token_address(&user_pubkey, &usdc_pubkey);

    let central_state_pubkey = Pubkey::from_str(PACIFICA_CENTRAL_STATE_ADDRESS).unwrap();
    let vault_pubkey = Pubkey::from_str(PACIFICA_VAULT_ADDRESS).unwrap();

    let token_program_pubkey = Pubkey::from_str(TOKEN_PROGRAM).unwrap();
    let system_program_pubkey = Pubkey::from_str(SYSTEM_PROGRAM).unwrap();

    let associated_token_program_pubkey = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap();
    let event_authority_pubkey = Pubkey::from_str(EVENT_AUTHORITY).unwrap();
    let pacifica_program_pubkey = Pubkey::from_str(PACIFICA_PROGRAM_ADDRESS).unwrap();

    let ix1 = Instruction {
        program_id: Pubkey::from_str(PACIFICA_PROGRAM_ADDRESS).unwrap(),
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

            d.extend_from_slice(&amount.to_le_bytes());

            d
        },
    };
}
