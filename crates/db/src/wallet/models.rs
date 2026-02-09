#[derive(Debug, Clone)]
pub struct Wallet {
    pub id: uuid::Uuid,
    pub turnkey_evm_address: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
