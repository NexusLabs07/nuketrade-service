use std::collections::HashMap;

use axum::{Json, extract::Path, http::StatusCode};
use hyperliquid::apis::user::UserInfo as HyperliquidUserInfo;
use pacifica::apis::user::UserInfo as PacificaUserInfo;
use serde::Deserialize;

use crate::types::{MergedPositionResponse, OpenPositionsResponse};

#[derive(Deserialize)]
pub struct MergedPositionsParams {
    pub user_evm_address: String,
    pub user_solana_address: String,
}

pub async fn get_merged_open_positions(
    Path(params): Path<MergedPositionsParams>,
) -> Result<Json<Vec<MergedPositionResponse>>, (StatusCode, String)> {
    let hl_client = HyperliquidUserInfo::new(Some(params.user_evm_address), None);
    let pacifica_client = PacificaUserInfo::new(params.user_solana_address);

    let (hl_result, pacifica_result) = tokio::join!(
        hl_client.get_open_positions(),
        pacifica_client.get_open_positions()
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
                pnl: pos.unrealized_pnl.clone(),
                funding: pos.cum_funding.all_time.clone(),
                leverage: pos.leverage.value.clone(),
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
        if pacifica_positions.success && pacifica_positions.data.is_some() {
            for asset_position in pacifica_positions.data.unwrap().iter() {
                let symbol = asset_position.symbol.clone();

                let pacifica_position = OpenPositionsResponse {
                    symbol: symbol.clone(),
                    size: asset_position.amount.clone(),
                    pnl: String::from("0"), //TODO
                    funding: asset_position.funding.clone(),
                    leverage: 0,                          //TODO
                    liquidation_price: String::from("0"), //TODO
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

    let merged_positions: Vec<MergedPositionResponse> = positions_map.into_values().collect();

    Ok(Json(merged_positions))
}
