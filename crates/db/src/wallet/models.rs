#[derive(Debug, Clone)]
pub struct Wallet {
    pub id: uuid::Uuid,
    pub turnkey_evm_address: String,
    pub turnkey_solana_address: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Per-user Hyperliquid agent wallet. One row per user once they've
/// completed the agent provisioning flow. The automation worker refuses
/// to place orders for a user without an `approved_on_hl=true` row here.
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct HlAgentWallet {
    pub user_id: uuid::Uuid,
    pub turnkey_suborg_id: String,
    pub turnkey_wallet_id: String,
    pub evm_address: String,
    pub approved_on_hl: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
