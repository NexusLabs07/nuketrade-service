use std::collections::HashMap;

use anyhow::Result;
use axum::{
    Json,
    extract::{Path, State},
};
use db::{crud::get_token_chart_info, types::FundingRate};
use hyperliquid::apis::user::{ClearinghouseState, UserInfo as HyperliquidUserInfo};
use pacifica::apis::user::{
    AccountSettingsResponse, UserInfo as PacificaUserInfo, UserPositionsResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    error::AppError,
    types::{MergedPositionResponse, OpenPositionsResponse},
};

#[derive(Deserialize)]
pub struct MergedPositionsParams {
    pub user_evm_address: String,
    pub user_solana_address: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FundingRateStruct {
    hyperliquid_funding_rate: Option<f64>,
    pacifica_funding_rate: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfoResponse {
    pub symbol: String,
    pub hyperliquid: Option<f64>,
    pub pacifica: Option<f64>,
}

pub async fn get_merged_open_positions(
    Path(params): Path<MergedPositionsParams>,
) -> Result<Json<Vec<MergedPositionResponse>>, AppError> {
    let hl_client = HyperliquidUserInfo::new(Some(params.user_evm_address), None);
    let pacifica_client = PacificaUserInfo::new(params.user_solana_address);

    let (hl_result, pacifica_result, pacifica_account_result): (
        Result<ClearinghouseState>,
        Result<UserPositionsResponse>,
        Result<AccountSettingsResponse>,
    ) = tokio::join!(
        hl_client.get_open_positions(),
        pacifica_client.get_open_positions(),
        pacifica_client.get_account_settings()
    );

    let mut positions_map: HashMap<String, MergedPositionResponse> = HashMap::new();

    // Process Hyperliquid positions
    if let Ok(hl_positions) = hl_result {
        for asset_position in hl_positions.asset_positions.iter() {
            let pos = &asset_position.position;
            let symbol = pos.coin.clone();

            let hl_position = OpenPositionsResponse {
                symbol: symbol.clone(),
                size: pos.szi.clone(),
                margin: pos.margin_used.clone(),
                pnl: pos.unrealized_pnl.clone(),
                funding: pos.cum_funding.all_time.clone(),
                leverage: pos.leverage.value,
                liquidation_price: pos.liquidation_px.clone().unwrap_or_default(),
            };

            positions_map
                .entry(symbol.clone())
                .or_insert_with(|| MergedPositionResponse {
                    symbol: symbol.clone(),
                    hyperliquid: None,
                    pacifica: None,
                })
                .hyperliquid = Some(hl_position);
        }
    }

    // Process Pacifica positions
    if let Ok(pacifica_positions) = pacifica_result {
        if let Some(positions_data) = pacifica_positions.data {
            if pacifica_positions.success {
                let account_settings = pacifica_account_result.ok().and_then(|r| r.data);

                for asset_position in positions_data.iter() {
                    let symbol = asset_position.symbol.clone();

                    let leverage: u32 = account_settings
                        .as_ref()
                        .and_then(|settings| settings.iter().find(|x| x.symbol == symbol))
                        .map(|s| s.leverage as u32)
                        .unwrap_or(0);

                    let margin = if asset_position.isolated {
                        asset_position.margin.clone().unwrap_or_default()
                    } else {
                        asset_position
                            .amount
                            .parse::<f64>()
                            .ok()
                            .map(|amt| {
                                if leverage > 0 {
                                    (amt / leverage as f64).to_string()
                                } else {
                                    "0".to_string()
                                }
                            })
                            .unwrap_or_else(|| "0".to_string())
                    };

                    let pacifica_position = OpenPositionsResponse {
                        symbol: symbol.clone(),
                        size: asset_position.amount.clone(),
                        pnl: String::from("0"), //TODO
                        funding: asset_position.funding.clone(),
                        leverage,
                        margin,
                        liquidation_price: asset_position.liquidation_price.clone(),
                    };

                    positions_map
                        .entry(symbol.clone())
                        .or_insert_with(|| MergedPositionResponse {
                            symbol: symbol.clone(),
                            hyperliquid: None,
                            pacifica: None,
                        })
                        .pacifica = Some(pacifica_position);
                }
            }
        }
    }

    let merged_positions: Vec<MergedPositionResponse> = positions_map.into_values().collect();

    Ok(Json(merged_positions))
}

pub async fn get_tokens_funding(
    State(state): State<AppState>,
) -> Result<Json<Vec<TokenInfoResponse>>, AppError> {
    let funding_rate = state.platforms_funding_rate.read().await;

    let mut tokens: HashMap<String, FundingRateStruct> = HashMap::new();

    for (symbol, rate) in funding_rate.hyperliquid.iter() {
        tokens
            .entry(symbol.clone())
            .or_default()
            .hyperliquid_funding_rate = Some(*rate);
    }

    for (symbol, rate) in funding_rate.pacifica.iter() {
        tokens
            .entry(symbol.clone())
            .or_default()
            .pacifica_funding_rate = Some(*rate);
    }

    let response: Vec<TokenInfoResponse> = tokens
        .into_iter()
        .map(|(symbol, rate)| TokenInfoResponse {
            symbol,
            hyperliquid: rate.hyperliquid_funding_rate,
            pacifica: rate.pacifica_funding_rate,
        })
        .collect();

    Ok(Json(response))
}

pub async fn get_token_chart(
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, Vec<FundingRate>>>, AppError> {
    let rows = get_token_chart_info(state.db, symbol).await?;

    let mut grouped: HashMap<String, Vec<FundingRate>> = HashMap::new();
    for row in rows {
        grouped.entry(row.platform.clone()).or_default().push(row);
    }

    Ok(Json(grouped))
}
