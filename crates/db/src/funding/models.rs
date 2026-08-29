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

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct AverageFundingStats {
    pub platform: String,
    pub symbol: String,
    pub avg_rate: f64,
    pub max_rate: f64,
    pub min_rate: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct HourlyFundingRate {
    pub symbol: String,
    pub platform: String,
    pub ts_hour: chrono::NaiveDateTime,
    pub rate: f64,
    /// Exclusive upper bound of the canonical trailing UTC-hour window.
    pub window_end_hour: chrono::NaiveDateTime,
}
