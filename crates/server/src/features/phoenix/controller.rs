use axum::{Json, extract::State};
use phoenix::{
    PhoenixExchange,
    apis::user::UserInfo,
    services::orders::{MarketOrderIxRequest, build_market_order_ix},
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedJson, ValidatedPath},
    services::PositionService,
    types::OpenPositionsResponse,
    validation::address::validate_solana_address,
};

#[derive(Deserialize, Validate)]
pub struct PhoenixUserPath {
    #[validate(custom(function = "validate_solana_address"))]
    pub user_address: String,
}

pub async fn get_user_open_positions(
    ValidatedPath(params): ValidatedPath<PhoenixUserPath>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    let client = UserInfo::with_pda_index(
        params.user_address,
        phoenix::helpers::collateral::DEFAULT_TRADER_PDA_INDEX,
    );
    let state = client.get_trader_state().await?;

    let mut out = Vec::new();
    for trader in state.traders {
        for pos in &trader.positions {
            if let Some(position) = PositionService::from_phoenix_position(pos, &trader) {
                out.push(position);
            }
        }
    }

    Ok(Json(out))
}

pub async fn get_perp_metadata(
    State(_state): State<AppState>,
) -> Result<Json<Vec<perp_core::exchange::MarketInfo>>, AppError> {
    let markets = PhoenixExchange::new().fetch_market_info().await?;
    Ok(Json(markets))
}

pub async fn build_market_order_transaction(
    ValidatedJson(payload): ValidatedJson<MarketOrderIxRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let response = build_market_order_ix(payload).await?;
    Ok(Json(serde_json::to_value(response)?))
}
