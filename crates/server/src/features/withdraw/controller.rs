use axum::Json;
use perp_core::exchange::PerpetualExchange;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawPayload {
    #[serde(rename = "sourceAddress")]
    pub source_address: Option<String>,
    #[serde(rename = "destinationAddress")]
    pub destination_address: Option<String>,
    #[serde(rename = "amount")]
    pub amount: String,
    pub exchange: PerpetualExchange,
}

pub async fn withdraw(Json(payload): Json<WithdrawPayload>) {
    match payload.exchange {
        PerpetualExchange::Hyperliquid => {
            hyperliquid::services::withdraw(payload.destination_address, payload.amount)
                .await
                .unwrap();
        }
        PerpetualExchange::Pacifica => {
            unimplemented!();
        }
        PerpetualExchange::Lighter => {
            // Lighter does not support withdraws for now
            unimplemented!();
        }
    }
}
