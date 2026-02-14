use axum::{
    Json,
    extract::{Path, State},
};
use pacifica::{
    apis::user::{AccountSettingsResponse, UserInfo, UserPositionsResponse},
    services::deposit::{DepositPayload, deposit_to_pacifica},
};

use crate::{
    AppState,
    error::AppError,
    types::{OpenPositionsResponse, Side},
};

pub async fn get_user_open_positions(
    Path(user_solana_address): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<OpenPositionsResponse>>, AppError> {
    let user_info_client = UserInfo::new(user_solana_address);

    let open_positions: UserPositionsResponse = user_info_client.get_open_positions().await?;
    let account_setting: AccountSettingsResponse = user_info_client.get_account_settings().await?;

    let mut open_position_response: Vec<OpenPositionsResponse> = Vec::new();

    let (Some(positions_data), Some(account_setting_data)) =
        (open_positions.data, account_setting.data)
    else {
        return Ok(Json(open_position_response));
    };

    if !open_positions.success || !account_setting.success {
        return Ok(Json(open_position_response));
    }

    let snapshot = state.feed.borrow().clone();

    for asset_position in positions_data.iter() {
        let leverage: u32 = account_setting_data
            .iter()
            .find(|x| x.symbol == asset_position.symbol)
            .and_then(|s| s.leverage.try_into().ok())
            .unwrap_or(0);

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
                Some(amt) if leverage > 0 => (amt * current_mark_px / leverage as f64).to_string(),
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

        open_position_response.push(OpenPositionsResponse {
            symbol: asset_position.symbol.clone(),
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
            margin,
            funding: asset_position.funding.clone().unwrap_or_default(),
            leverage,
            liquidation_price: asset_position.liquidation_price.clone().unwrap_or_default(),
        });
    }

    Ok(Json(open_position_response))
}

pub async fn bridge_to_pacifica(
    State(state): State<AppState>,
    Json(payload): Json<DepositPayload>,
) -> Result<Json<String>, AppError> {
    let solana_rpc_url = state.config.solana_rpc_url;
    let fee_payer_private_key = state.config.solana_fee_payer_private_key;

    let serialized_tx = deposit_to_pacifica(solana_rpc_url, fee_payer_private_key, payload).await?;

    Ok(Json(serialized_tx))
}
