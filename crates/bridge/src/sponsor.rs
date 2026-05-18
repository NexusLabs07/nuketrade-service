use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde_json::Value;
use std::str::FromStr;

use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::v0::Message as MessageV0;
use solana_message::{AddressLookupTableAccount, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

// TODO(billing): Charge the user back for the SOL gas this fee-payer absorbs.
// Mirror Pacifica's `GAS_REIMBURSEMENT_AMOUNT = 200_000` (0.2 USDC) by injecting a
// pre-instruction that transfers a fixed USDC amount from the user's USDC ATA to the
// fee-payer's USDC ATA before the bridge instruction(s). For now we eat the cost.

/// Walks a Relay `/quote/v2` response's `steps` array. For every `kind = "transaction"`
/// item, rebuilds the Solana v0 transaction with our fee-payer hot wallet as payer,
/// partial-signs as fee-payer, and inserts a base64-encoded partially-signed
/// `VersionedTransaction` under `data.sponsoredTransaction`. The original
/// `instructions` / `addressLookupTableAddresses` fields are preserved so the FE can
/// fall back if it ignores the new field.
///
/// Returns the number of transaction items sponsored.
pub async fn sponsor_solana_steps(
    steps: &mut Value,
    rpc_url: &str,
    fee_payer_private_key_b58: &str,
) -> Result<usize> {
    let steps_array = steps
        .as_array_mut()
        .ok_or_else(|| anyhow!("Relay steps payload is not a JSON array"))?;

    let rpc = RpcClient::new(rpc_url.to_string());
    let fee_payer = Keypair::from_base58_string(fee_payer_private_key_b58);

    let mut sponsored_count = 0usize;

    for step in steps_array.iter_mut() {
        let kind = step.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        if kind != "transaction" {
            continue;
        }

        let Some(items) = step.get_mut("items").and_then(|i| i.as_array_mut()) else {
            continue;
        };

        for item in items.iter_mut() {
            let Some(data) = item.get_mut("data") else {
                continue;
            };

            match sponsor_one_item(data, &rpc, &fee_payer).await {
                Ok(b64_tx) => {
                    if let Some(obj) = data.as_object_mut() {
                        obj.insert("sponsoredTransaction".to_string(), Value::String(b64_tx));
                    }
                    sponsored_count += 1;
                }
                Err(e) => {
                    log::warn!("sponsor_solana_steps: failed to sponsor an item: {e:#}");
                    return Err(e);
                }
            }
        }
    }

    Ok(sponsored_count)
}

async fn sponsor_one_item(data: &Value, rpc: &RpcClient, fee_payer: &Keypair) -> Result<String> {
    let instructions_json = data
        .get("instructions")
        .and_then(|i| i.as_array())
        .ok_or_else(|| anyhow!("transaction item is missing `instructions` array"))?;

    let instructions = instructions_json
        .iter()
        .map(parse_instruction)
        .collect::<Result<Vec<Instruction>>>()
        .context("failed to parse Relay instructions")?;

    let alt_addresses: Vec<Pubkey> = data
        .get("addressLookupTableAddresses")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(Pubkey::from_str)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .context("failed to parse addressLookupTableAddresses")?
        .unwrap_or_default();

    let alts = if alt_addresses.is_empty() {
        Vec::new()
    } else {
        fetch_alts(rpc, &alt_addresses).await?
    };

    let recent_blockhash = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .context("failed to fetch recent blockhash")?
        .0;

    let v0_msg =
        MessageV0::try_compile(&fee_payer.pubkey(), &instructions, &alts, recent_blockhash)
            .context("MessageV0::try_compile failed")?;

    let message = VersionedMessage::V0(v0_msg);
    let num_signers = message.header().num_required_signatures as usize;
    let mut signatures = vec![Signature::default(); num_signers];

    let serialized_msg = message.serialize();
    let fee_payer_sig = fee_payer.sign_message(&serialized_msg);
    // Fee payer is always the first required signer in a Solana message.
    signatures[0] = fee_payer_sig;

    let tx = VersionedTransaction {
        signatures,
        message,
    };
    let bytes = bincode::serialize(&tx).context("failed to serialize VersionedTransaction")?;
    Ok(BASE64.encode(bytes))
}

fn parse_instruction(value: &Value) -> Result<Instruction> {
    let program_id_str = value
        .get("programId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("instruction missing `programId`"))?;
    let program_id = Pubkey::from_str(program_id_str)
        .with_context(|| format!("invalid programId: {program_id_str}"))?;

    let keys = value
        .get("keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("instruction missing `keys`"))?;

    let accounts = keys
        .iter()
        .map(|k| {
            let pubkey_str = k
                .get("pubkey")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("key entry missing `pubkey`"))?;
            let pubkey = Pubkey::from_str(pubkey_str)
                .with_context(|| format!("invalid key pubkey: {pubkey_str}"))?;
            let is_signer = k.get("isSigner").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_writable = k
                .get("isWritable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(if is_writable {
                AccountMeta::new(pubkey, is_signer)
            } else {
                AccountMeta::new_readonly(pubkey, is_signer)
            })
        })
        .collect::<Result<Vec<AccountMeta>>>()?;

    let data_str = value
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("instruction missing `data`"))?;
    let data = decode_instruction_data(data_str)?;

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

fn decode_instruction_data(s: &str) -> Result<Vec<u8>> {
    // Relay encodes Solana instruction `data` as a hex string (no `0x` prefix).
    let trimmed = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(trimmed)
        .with_context(|| format!("instruction data is not valid hex: {s}"))
}

async fn fetch_alts(
    rpc: &RpcClient,
    addresses: &[Pubkey],
) -> Result<Vec<AddressLookupTableAccount>> {
    let accounts = rpc
        .get_multiple_accounts_with_commitment(addresses, CommitmentConfig::confirmed())
        .await
        .context("failed to fetch ALT accounts")?
        .value;

    let mut out = Vec::with_capacity(addresses.len());
    for (idx, account_opt) in accounts.into_iter().enumerate() {
        let account = account_opt
            .ok_or_else(|| anyhow!("ALT account {} not found on-chain", addresses[idx]))?;
        let table = AddressLookupTable::deserialize(&account.data)
            .map_err(|e| anyhow!("failed to deserialize ALT {}: {e}", addresses[idx]))?;
        out.push(AddressLookupTableAccount {
            key: addresses[idx],
            addresses: table.addresses.into_owned(),
        });
    }
    Ok(out)
}
