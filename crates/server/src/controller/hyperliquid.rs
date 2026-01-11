use axum::Json;
use hyperliquid::PerpOrderRequest;
use serde_json::Value;

use crate::error::AppError;

//TODO: Add types instead of !
pub async fn create_perp_position(
    Json(perp_order_request): Json<PerpOrderRequest>,
) -> Result<Json<Value>, AppError> {
    let create_perp_position_data =
        hyperliquid::create_perp_position_typed_data(perp_order_request).await?;

    Ok(Json(create_perp_position_data))
}
pub async fn close_perp_position(
    Json(perp_order_request): Json<PerpOrderRequest>,
) -> Result<Json<Value>, AppError> {
    let close_perp_position_data =
        hyperliquid::close_perp_position_typed_data(perp_order_request).await?;

    Ok(Json(close_perp_position_data))
}

pub async fn close_all_perp_position(
    Json(orders_request): Json<Vec<PerpOrderRequest>>,
) -> Result<Json<Vec<Value>>, AppError> {
    let close_all_perp_position_data =
        hyperliquid::close_all_perp_position_typed_data(orders_request).await?;

    Ok(Json(close_all_perp_position_data))
}

pub async fn cancel_perp_order(
    Json(cancel_order_request): Json<hyperliquid::CancelOrderRequest>,
) -> Result<Json<Value>, AppError> {
    let cancel_order_data = hyperliquid::cancel_order_typed_data(cancel_order_request).await?;

    Ok(Json(cancel_order_data))
}
