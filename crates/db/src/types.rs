use sqlx::types::chrono;

pub struct FundingRate {
    pub id: uuid::Uuid,
    pub platform: String,
    pub symbol: String,
    pub rate: f64,
    pub mark_px: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
