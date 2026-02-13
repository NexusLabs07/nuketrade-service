use std::collections::HashMap;

use anyhow::Result;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use db::funding::{FundingRate, get_token_chart_info};
use hyperliquid::{
    apis::user::{ClearinghouseState, UserInfo as HyperliquidUserInfo},
    // helpers::markets::HL_MARKETS,
};
use pacifica::{
    apis::user::{AccountSettingsResponse, UserInfo as PacificaUserInfo, UserPositionsResponse},
    helpers::markets::PACIFICA_MARKETS,
};
use perp_core::{SevenDayApr, parse_f64_or_zero};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    AppState,
    error::AppError,
    middleware::user::{validate_evm_address, validate_solana_address, validate_timeframe},
    types::{
        LiveMarketFeedResponse, MarketFeedValueStruct, MergedPositionResponse,
        OpenPositionsResponse, Side,
    },
};

#[derive(Deserialize, Validate)]
pub struct MergedPositionsParams {
    #[validate(custom(function = "validate_evm_address"))]
    pub user_evm_address: String,
    #[validate(custom(function = "validate_solana_address"))]
    pub user_solana_address: String,
}

//Market feeed with price and funding rate
#[derive(Debug, Default, Serialize, Deserialize)]
struct MarketFeedStruct {
    hyperliquid: MarketFeedValueStruct,
    pacifica: MarketFeedValueStruct,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ChartParams {
    #[validate(custom(function = "validate_timeframe"))]
    timeframe: String,
}

pub async fn get_merged_open_positions(
    Path(params): Path<MergedPositionsParams>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MergedPositionResponse>>, AppError> {
    params.validate()?;

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

    if let Ok(hl_positions) = hl_result {
        for asset_position in hl_positions.asset_positions.iter() {
            let pos = &asset_position.position;
            let symbol = pos.coin.clone();
            let size_value = parse_f64_or_zero(&pos.szi);
            let side = if size_value > 0.0 {
                Side::Long
            } else {
                Side::Short
            };

            let hl_position = OpenPositionsResponse {
                symbol: symbol.clone(),
                size: if side == Side::Short {
                    (-size_value).to_string()
                } else {
                    pos.szi.clone()
                },
                side,
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

    if let Ok(pacifica_positions) = pacifica_result {
        if let Some(positions_data) = pacifica_positions.data {
            if pacifica_positions.success {
                let account_settings = pacifica_account_result.ok().and_then(|r| r.data);
                let snapshot = state.feed.borrow().clone();

                for asset_position in positions_data.iter() {
                    let symbol = asset_position.symbol.clone();

                    let leverage: u32 = account_settings
                        .as_ref()
                        .and_then(|settings| settings.iter().find(|x| x.symbol == symbol))
                        .map(|s| s.leverage as u32)
                        .unwrap_or(
                            PACIFICA_MARKETS
                                .iter()
                                .find(|x| x.symbol == symbol)
                                .map(|s| s.max_leverage)
                                .unwrap_or_default(),
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
                            _ => "0".to_string(),
                        }
                    };

                    let pnl: f64 = if current_mark_px != 0.0 {
                        let entry_price = asset_position.entry_price.parse::<f64>().unwrap_or(0.0);
                        let amount = asset_position.amount.parse::<f64>().unwrap_or(0.0);
                        (current_mark_px - entry_price) * amount
                    } else {
                        0.0
                    };

                    let pacifica_position = OpenPositionsResponse {
                        symbol: symbol.clone(),
                        size: asset_position.amount.clone(),
                        side: if asset_position.side == "bid" {
                            Side::Long
                        } else {
                            Side::Short
                        },
                        pnl: if asset_position.side == "ask" {
                            (-pnl).to_string()
                        } else {
                            pnl.to_string()
                        },
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

pub async fn get_live_market_feed(
    State(state): State<AppState>,
) -> Result<Json<Vec<LiveMarketFeedResponse>>, AppError> {
    // let live_market_feed = state.live_market_feed.read().await.clone();

    // let mut market_feed: HashMap<String, MarketFeedStruct> = HashMap::new();

    // for (symbol, (mark_px, funding_rate)) in live_market_feed.hyperliquid.iter() {
    //     let max_leverage = HL_MARKETS
    //         .iter()
    //         .find(|x| x.name == *symbol)
    //         .map(|x| x.max_leverage);

    //     market_feed.entry(symbol.clone()).or_default().hyperliquid = MarketFeedValueStruct {
    //         mark_px: Some(*mark_px),
    //         funding: Some(*funding_rate),
    //         max_leverage,
    //     };
    // }

    // for (symbol, (mark_px, funding_rate)) in live_market_feed.pacifica.iter() {
    //     let max_leverage = PACIFICA_MARKETS
    //         .iter()
    //         .find(|x| x.symbol == symbol)
    //         .map(|x| x.max_leverage);

    //     market_feed.entry(symbol.clone()).or_default().pacifica = MarketFeedValueStruct {
    //         mark_px: Some(*mark_px),
    //         funding: Some(*funding_rate),
    //         max_leverage,
    //     };
    // }

    // let response: Vec<LiveMarketFeedResponse> = market_feed
    //     .into_iter()
    //     .map(|(symbol, market_feed)| {
    //         let hyperliquid = if market_feed.hyperliquid.mark_px.is_some() {
    //             Some(market_feed.hyperliquid)
    //         } else {
    //             None
    //         };
    //         let pacifica = if market_feed.pacifica.mark_px.is_some() {
    //             Some(market_feed.pacifica)
    //         } else {
    //             None
    //         };
    //         LiveMarketFeedResponse {
    //             symbol,
    //             hyperliquid,
    //             pacifica,
    //         }
    //     })
    //     .collect();

    // Ok(Json(response))

    // clone the formatted snapshot from FeedManager
    let snapshot = state.feed.borrow().clone();
    Ok(Json(snapshot.formatted.clone()))
}

pub async fn get_token_chart(
    Path(symbol): Path<String>,
    Query(params): Query<ChartParams>,
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, Vec<FundingRate>>>, AppError> {
    params.validate()?;

    let rows = get_token_chart_info(state.db, symbol, params.timeframe).await?;

    let mut grouped: HashMap<String, Vec<FundingRate>> = HashMap::new();
    for row in rows {
        grouped.entry(row.platform.clone()).or_default().push(row);
    }

    Ok(Json(grouped))
}

pub async fn get_average_apr(State(state): State<AppState>) -> Result<Json<SevenDayApr>, AppError> {
    let seven_day_apr = state.seven_day_apr.borrow().clone();
    Ok(Json(seven_day_apr))
}
