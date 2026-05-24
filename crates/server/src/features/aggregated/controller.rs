use std::collections::HashMap;

use anyhow::Result;
use axum::{
    Json,
    extract::{Path, State},
};
use db::funding::{FundingRate, get_token_chart_info};
use hyperliquid::apis::user::{ClearinghouseState, UserFill, UserInfo as HyperliquidUserInfo};
use pacifica::apis::user::{
    AccountSettingsResponse, UserInfo as PacificaUserInfo, UserPositionsHistoryResponse,
    UserPositionsResponse,
};
use perp_core::{SevenDayApr, token_list::TOKEN_LIST};
use phoenix::apis::user::{
    TradeHistoryResponse as PhoenixTradeHistoryResponse, TraderStateResponse as PhoenixTraderState,
    UserInfo as PhoenixUserInfo,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    extractors::{ValidatedPath, ValidatedQuery},
    services::PositionService,
    types::{
        LiveMarketFeedResponse, MergedClosedPositionResponse, MergedPositionResponse,
        OpenPositionsResponse,
    },
    validation::address::{validate_evm_address, validate_solana_address, validate_timeframe},
};

#[derive(Deserialize, Validate)]
pub struct MergedPositionsParams {
    #[validate(custom(function = "validate_evm_address"))]
    pub user_evm_address: String,
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChartParams {
    #[validate(custom(function = "validate_timeframe"))]
    timeframe: String,
}

fn is_supported_token(symbol: &str) -> bool {
    TOKEN_LIST.contains(&symbol)
}

pub async fn get_merged_open_positions(
    ValidatedPath(params): ValidatedPath<MergedPositionsParams>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MergedPositionResponse>>, AppError> {
    let hl_client = HyperliquidUserInfo::new(Some(params.user_evm_address), None);
    let pacifica_client = PacificaUserInfo::new(params.user_solana_address.clone());
    let phoenix_client = PhoenixUserInfo::with_pda_index(
        params.user_solana_address.clone(),
        phoenix::helpers::collateral::DEFAULT_TRADER_PDA_INDEX,
    );

    let (hl_result, pacifica_result, pacifica_account_result, phoenix_result): (
        Result<ClearinghouseState>,
        Result<UserPositionsResponse>,
        Result<AccountSettingsResponse>,
        Result<PhoenixTraderState>,
    ) = tokio::join!(
        hl_client.get_open_positions(),
        pacifica_client.get_open_positions(),
        pacifica_client.get_account_settings(),
        phoenix_client.get_trader_state(),
    );

    let mut hl_positions_vec: Vec<OpenPositionsResponse> = Vec::new();
    if let Ok(hl_positions) = hl_result {
        for asset_position in &hl_positions.asset_positions {
            hl_positions_vec.push(PositionService::from_hyperliquid_position(
                &asset_position.position,
            ));
        }
    }

    let mut pacifica_positions_vec: Vec<OpenPositionsResponse> = Vec::new();
    if let Ok(pacifica_positions) = pacifica_result {
        if let Some(positions_data) = pacifica_positions.data {
            if pacifica_positions.success {
                let account_resp = pacifica_account_result.ok();
                let snapshot = state.feed.borrow().clone();

                for asset_position in &positions_data {
                    let leverage = PositionService::resolve_pacifica_leverage(
                        account_resp.as_ref().and_then(|r| r.margin_settings()),
                        &asset_position.symbol,
                    );

                    let current_mark_px = snapshot
                        .by_symbol
                        .get(&asset_position.symbol)
                        .and_then(|feed| feed.pacifica.as_ref())
                        .and_then(|value| value.mark_px)
                        .unwrap_or(0.0);

                    let margin = if asset_position.isolated {
                        asset_position.margin.clone().unwrap_or_default()
                    } else {
                        match asset_position.amount.parse::<f64>().ok() {
                            Some(amt) if leverage > 0 => {
                                (amt * current_mark_px / leverage as f64).to_string()
                            }
                            _ => asset_position
                                .margin
                                .clone()
                                .filter(|m| !m.is_empty())
                                .unwrap_or_else(|| "0".to_string()),
                        }
                    };

                    let pnl: f64 = if current_mark_px != 0.0 {
                        let entry_price = asset_position.entry_price.parse::<f64>().unwrap_or(0.0);
                        let amount = asset_position.amount.parse::<f64>().unwrap_or(0.0);
                        (current_mark_px - entry_price) * amount
                    } else {
                        0.0
                    };

                    pacifica_positions_vec.push(
                        PositionService::from_pacifica_position_with_metrics(
                            asset_position,
                            leverage,
                            margin,
                            pnl,
                        ),
                    );
                }
            }
        }
    }

    let phoenix_positions_vec = match phoenix_result {
        Ok(state) => state
            .traders
            .iter()
            .flat_map(|trader| trader.positions.iter())
            .filter_map(|pos| PositionService::from_phoenix_position(pos))
            .collect::<Vec<_>>(),
        Err(err) => {
            log::warn!(
                "phoenix trader state failed for open positions ({}): {err}",
                params.user_solana_address
            );
            Vec::new()
        }
    };

    let merged_positions = PositionService::merge_positions(
        hl_positions_vec,
        pacifica_positions_vec,
        phoenix_positions_vec,
    );

    Ok(Json(merged_positions))
}

pub async fn get_merged_closed_positions(
    ValidatedPath(params): ValidatedPath<MergedPositionsParams>,
) -> Result<Json<Vec<MergedClosedPositionResponse>>, AppError> {
    let hl_client = HyperliquidUserInfo::new(Some(params.user_evm_address), None);
    let pacifica_client = PacificaUserInfo::new(params.user_solana_address.clone());
    let phoenix_client = PhoenixUserInfo::new(params.user_solana_address);

    let (hl_result, pacifica_result, phoenix_result): (
        Result<Vec<UserFill>>,
        Result<UserPositionsHistoryResponse>,
        Result<PhoenixTradeHistoryResponse>,
    ) = tokio::join!(
        hl_client.get_closed_positions(),
        pacifica_client.get_closed_positions(),
        phoenix_client.get_trade_history(),
    );

    let hl_closed_positions = hl_result
        .unwrap_or_default()
        .iter()
        .filter_map(PositionService::from_hyperliquid_closed_fill)
        .collect::<Vec<_>>();

    let pacifica_closed_positions = pacifica_result
        .ok()
        .and_then(|history| if history.success { history.data } else { None })
        .unwrap_or_default()
        .iter()
        .filter_map(PositionService::from_pacifica_closed_position)
        .collect::<Vec<_>>();

    let phoenix_closed_positions = phoenix_result
        .ok()
        .map(|history| history.data)
        .unwrap_or_default()
        .iter()
        .filter_map(PositionService::from_phoenix_closed_trade)
        .collect::<Vec<_>>();

    let merged_positions = PositionService::merge_closed_positions(
        hl_closed_positions,
        pacifica_closed_positions,
        phoenix_closed_positions,
    );

    Ok(Json(merged_positions))
}

pub async fn get_live_market_feed(
    State(state): State<AppState>,
) -> Result<Json<Vec<LiveMarketFeedResponse>>, AppError> {
    let snapshot = state.feed.borrow().clone();

    let filtered: Vec<LiveMarketFeedResponse> = snapshot
        .formatted
        .iter()
        .filter(|entry| is_supported_token(&entry.symbol))
        .cloned()
        .collect();
    Ok(Json(filtered))
}

pub async fn get_token_chart(
    Path(symbol): Path<String>,
    ValidatedQuery(params): ValidatedQuery<ChartParams>,
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, Vec<FundingRate>>>, AppError> {
    if !is_supported_token(&symbol) {
        return Ok(Json(HashMap::new()));
    }

    let mut rows = get_token_chart_info(state.db, symbol, params.timeframe).await?;

    for row in &mut rows {
        if row.platform == phoenix::helpers::funding::PLATFORM {
            row.rate = phoenix::helpers::funding::normalize_stored_hourly_rate(row.rate);
        }
    }

    let mut grouped: HashMap<String, Vec<FundingRate>> = HashMap::new();
    for row in rows {
        grouped.entry(row.platform.clone()).or_default().push(row);
    }

    Ok(Json(grouped))
}

pub async fn get_average_apr(State(state): State<AppState>) -> Result<Json<SevenDayApr>, AppError> {
    let seven_day_apr = state.seven_day_apr.borrow().clone();

    let filtered = SevenDayApr {
        seven_day_avg_apr: seven_day_apr
            .seven_day_avg_apr
            .into_iter()
            .filter(|(symbol, _)| is_supported_token(symbol))
            .collect(),
        seven_day_spread_apr: seven_day_apr
            .seven_day_spread_apr
            .into_iter()
            .filter(|(symbol, _)| is_supported_token(symbol))
            .collect(),
    };

    Ok(Json(filtered))
}
