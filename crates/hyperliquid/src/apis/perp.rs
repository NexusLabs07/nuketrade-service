use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpOrderRequest {
    pub asset_index: u32,
    pub asset_name: String,
    pub price: Option<f64>,
    pub size: String,
    pub is_market: Option<bool>,
    pub vault_address: Option<String>,
    pub is_long: Option<bool>,
}

pub async fn create_perp_position_typed_data(order_request: PerpOrderRequest) {
    let PerpOrderRequest {
        price,
        size,
        asset_index,
        asset_name,
        is_long,
        is_market,
        vault_address,
    } = order_request;

    if is_market.is_none() && price.is_none() {
        //TODO: Return an error back here
    }
}
