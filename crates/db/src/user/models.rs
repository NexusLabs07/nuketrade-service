#[derive(Debug, Clone)]
pub struct UserPayload {
    pub id: uuid::Uuid,
    pub email: Option<String>,
    pub connected_evm_address: Option<String>,
    pub connected_solana_address: Option<String>,
    pub referred_by: Option<uuid::Uuid>,
    pub turnkey_evm_address: String,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: uuid::Uuid,
    pub email: Option<String>,
    pub name: String,
    pub connected_evm_address: Option<String>,
    pub connected_solana_address: Option<String>,
    pub referral_code: String,
    pub referred_by: Option<uuid::Uuid>,
    pub wallet_id: uuid::Uuid,
    pub is_pacifica_access_claimed: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
