use axum::{
    Json,
    extract::{Path, State},
};
use pacifica::apis::user::{AccountSettingsResponse, UserInfo, UserPositionsResponse};

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

    for asset_position in positions_data.iter() {
        let leverage: u32 = account_setting_data
            .iter()
            .find(|x| x.symbol == asset_position.symbol)
            .and_then(|s| s.leverage.try_into().ok())
            .unwrap_or(0);

        let current_feed = state
            .live_market_feed
            .read()
            .await
            .pacifica
            .get(&asset_position.symbol)
            .cloned()
            .unwrap_or_default();

        let margin = if asset_position.isolated {
            asset_position.margin.clone().unwrap_or_default()
        } else {
            let value = match asset_position.amount.parse::<f64>().ok() {
                Some(amt) if leverage > 0 => (amt * current_feed.0 / leverage as f64).to_string(),
                _ => "0".to_string(),
            };

            value
        };

        let pnl: f64 = if current_feed.0 != 0.0 {
            let entry_price = asset_position.entry_price.parse::<f64>().unwrap_or(0.0);
            let amount = asset_position.amount.parse::<f64>().unwrap_or(0.0);

            (current_feed.0 - entry_price) * amount
        } else {
            0.0
        };

        // let pnl = asset_position.entry_price
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
            funding: asset_position.funding.clone(),
            leverage,
            liquidation_price: asset_position.liquidation_price.clone(),
        });
    }

    Ok(Json(open_position_response))
}
