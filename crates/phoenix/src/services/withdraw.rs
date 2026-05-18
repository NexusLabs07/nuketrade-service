use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawPayload {
    pub authority: String,
    pub amount: String,
}

/// Placeholder for now
/// Phoenix withdrawals are Solana transaction based.
/// The actual transaction builder should be wired through the Phoenix Rise SDK
/// or the external executor, because the user must sign the transaction.
pub async fn withdraw_from_phoenix(_payload: WithdrawPayload) -> anyhow::Result<serde_json::Value> {
    anyhow::bail!("Phoenix withdraw transaction builder is not wired yet")
}
