use axum::Json;
use lighter::LighterExchange;
use perp_core::MarketInfo;

use crate::error::AppError;

pub async fn get_perp_metadata() -> Result<Json<Vec<MarketInfo>>, AppError> {
    let exchange = LighterExchange::new();
    let markets = exchange.fetch_active_perp_markets().await?;

    let response = markets
        .into_iter()
        .map(|market| MarketInfo {
            symbol: market.symbol,
            max_leverage: market.max_leverage,
            tick_size: market.tick_size,
            min_order_size: market.min_order_size,
            size_decimals: market.size_decimals,
            is_active: true,
            exchange_id: Some(market.market_index),
        })
        .collect();

    Ok(Json(response))
}
