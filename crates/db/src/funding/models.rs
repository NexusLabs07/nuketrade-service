use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct FundingRate {
    pub id: uuid::Uuid,
    pub platform: String,
    pub symbol: String,
    pub rate: f64,
    pub mark_px: f64,
    pub timestamp: chrono::NaiveDateTime,
}
