use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositPayload {
    pub authority: String,
    pub amount: String,
}

/// Placeholder for now
/// Phoenix deposits are Solana transaction based.
/// The actual transaction builder should be wired through the Phoenix Rise SDK
/// or the external executor, because the user must sign the transaction.
pub async fn deposit_to_phoenix(_payload: DepositPayload) -> anyhow::Result<serde_json::Value> {
    anyhow::bail!("Phoenix deposit transaction builder is not wired yet")
}
