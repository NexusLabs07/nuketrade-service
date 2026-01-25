use axum::{Json, extract::Path};
use pacifica::apis::user::{AccountSettingsResponse, UserInfo, UserPositionsResponse};

use crate::{error::AppError, types::OpenPositionsResponse};

pub async fn get_user_open_positions(
    Path(user_solana_address): Path<String>,
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

        open_position_response.push(OpenPositionsResponse {
            symbol: asset_position.symbol.clone(),
            size: asset_position.amount.clone(),
            pnl: String::from("0"), //TODO
            margin: asset_position.margin.clone().unwrap_or_default(),
            funding: asset_position.funding.clone(),
            leverage,
            liquidation_price: asset_position.liquidation_price.clone(),
        });
    }

    Ok(Json(open_position_response))
}
